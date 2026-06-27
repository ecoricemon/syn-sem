//! Initial type relation equality fact collection.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{InferTypes, PrimitiveType, Type, TypeId};
use syn_sem_common::{Map, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name as name;

pub(crate) struct TypeRelationCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a name::NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
    equalities: Vec<TypeEqualityFact>,
}

impl<'a, 'cx> TypeRelationCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a name::NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> TypeRelationDb {
        Self {
            hir,
            names,
            types,
            equalities: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> TypeRelationDb {
        self.collect_signature_facts();
        self.collect_expr_facts();
        self.collect_lowered_block_facts();

        TypeRelationDb {
            equalities: self.equalities,
            resolved: Vec::new(),
            expr_types: Map::default(),
            def_types: Map::default(),
        }
    }

    fn collect_signature_facts(&mut self) {
        for signature in self.hir.signatures() {
            for param in signature.params.iter().skip(1) {
                if let Some(pat) = param.pat {
                    self.collect_pat_type_facts(pat, param.ty);
                }
            }
        }
    }

    fn collect_expr_facts(&mut self) {
        for expr in self.hir.exprs() {
            match &expr.kind {
                hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                    if let Some(tail_expr) = self.hir.lowered_blocks()[*block].tail_expr {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Expr(tail_expr),
                        );
                    }
                }
                hir::ExprKind::Cast { ty, .. } => {
                    self.intern_type_equality(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Type(
                            self.types
                                .type_for_hir_type(*ty)
                                .expect("HIR types are lowered before type relations"),
                        ),
                    );
                }
                hir::ExprKind::Lit(lit) => {
                    if let Some(ty) = self.lit_type(lit) {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Type(ty),
                        );
                    }
                }
                hir::ExprKind::Paren { expr: inner } => {
                    self.intern_type_equality(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Expr(*inner),
                    );
                }
                hir::ExprKind::Path(path) => {
                    if let Some(def) =
                        Self::resolve_value_path(self.names, expr.scope, &path.segments)
                    {
                        self.intern_type_equality(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Def(def),
                        );
                    }
                }
                hir::ExprKind::Array { .. }
                | hir::ExprKind::Assign { .. }
                | hir::ExprKind::Binary { .. }
                | hir::ExprKind::Call { .. }
                | hir::ExprKind::Closure { .. }
                | hir::ExprKind::Field { .. }
                | hir::ExprKind::Index { .. }
                | hir::ExprKind::MethodCall { .. }
                | hir::ExprKind::Reference { .. }
                | hir::ExprKind::Repeat { .. }
                | hir::ExprKind::Return { .. }
                | hir::ExprKind::Struct { .. }
                | hir::ExprKind::Tuple { .. }
                | hir::ExprKind::Unary { .. } => {}
            }
        }

        for item in self.hir.items() {
            let hir::ItemKind::Fn {
                signature, block, ..
            } = item.kind
            else {
                continue;
            };
            let Some(tail_expr) = self.hir.lowered_blocks()[block].tail_expr else {
                continue;
            };
            let Some(return_param) = self.hir[signature].params.first() else {
                continue;
            };
            let Some(return_ty_id) = self.types.type_for_hir_type(return_param.ty) else {
                continue;
            };
            self.intern_type_equality(
                TypeSubject::Expr(tail_expr),
                TypeSubject::Type(return_ty_id),
            );
        }
    }

    fn collect_lowered_block_facts(&mut self) {
        for block in self.hir.lowered_blocks().blocks() {
            for stmt in &block.stmts {
                let hir::lower::Stmt::Local(local) = stmt else {
                    continue;
                };
                self.collect_local_facts(local);
            }
        }
    }

    fn collect_local_facts(&mut self, local: &hir::lower::Local) {
        if let Some(init) = local.init {
            self.bind_pat_to_expr(local.pat, init);
        }
    }

    fn collect_pat_type_facts(&mut self, pat: hir::PatId, hir_ty_id: hir::TypeId) {
        if let Some(ty) = self.types.type_for_hir_type(hir_ty_id) {
            self.bind_pat_to_type(pat, ty);
        }
    }

    fn bind_pat_to_type(&mut self, pat: hir::PatId, ty: TypeId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.intern_type_equality(TypeSubject::Def(*def), TypeSubject::Type(ty));
            }
            hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
                self.bind_pat_to_type(*pat, ty);
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    fn bind_pat_to_expr(&mut self, pat: hir::PatId, expr: hir::ExprId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.intern_type_equality(TypeSubject::Def(*def), TypeSubject::Expr(expr));
            }
            hir::PatKind::Reference { pat, .. } => self.bind_pat_to_expr(*pat, expr),
            hir::PatKind::Type { pat, ty } => {
                let Some(ty) = self.types.type_for_hir_type(*ty) else {
                    return;
                };
                self.bind_pat_to_type(*pat, ty);
                self.intern_type_equality(TypeSubject::Expr(expr), TypeSubject::Type(ty));
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    fn lit_type(&mut self, lit: &hir::Lit<'cx>) -> Option<TypeId> {
        match lit {
            hir::Lit::Bool(_) => Some(self.types.intern_type(Type::Primitive(PrimitiveType::Bool))),
            hir::Lit::Int(_) => Some(
                self.types
                    .insert_fresh_type(Type::Primitive(PrimitiveType::AbstractInt)),
            ),
            hir::Lit::Float(_) => Some(
                self.types
                    .insert_fresh_type(Type::Primitive(PrimitiveType::AbstractFloat)),
            ),
        }
    }

    fn intern_type_equality(&mut self, left: TypeSubject, right: TypeSubject) {
        let fact = TypeEqualityFact { left, right };
        self.equalities.push_unique(fact);
    }

    fn resolve_value_path(
        names: &name::NameDb<'cx>,
        scope: Option<syn_sem_name::ScopeId>,
        path: &[hir::PathSegment<'cx>],
    ) -> Option<syn_sem_name::DefId> {
        let scope = scope?;
        match names.resolve_value_path(scope, path.iter().map(|segment| segment.name)) {
            name::ResolveResult::Found(def) => Some(def),
            name::ResolveResult::Ambiguous(_) | name::ResolveResult::NotFound => None,
        }
    }
}
