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
                hir::ExprKind::Call { func, args } => {
                    self.collect_call_facts(expr.id, *func, args);
                }
                hir::ExprKind::Array { .. }
                | hir::ExprKind::Assign { .. }
                | hir::ExprKind::Binary { .. }
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

        self.collect_fn_return_facts();
    }

    fn collect_call_facts(&mut self, call: hir::ExprId, func: hir::ExprId, args: &[hir::ExprId]) {
        let hir::ExprKind::Path(path) = &self.hir[func].kind else {
            return;
        };
        let Some(def) = Self::resolve_value_path(self.names, self.hir[func].scope, &path.segments)
        else {
            return;
        };
        let Some(signature) = self.fn_signature_for_def(def) else {
            return;
        };
        let params = &self.hir[signature].params;
        let Some(return_param) = params.first() else {
            return;
        };
        let input_params = &params[1..];
        if args.len() != input_params.len() {
            return;
        }
        // Multi-argument call facts can currently make transitive type-relation queries grow
        // too much; keep the first call slice to zero- and one-argument functions.
        if input_params.len() > 1 {
            return;
        }

        let Some(return_ty) = self.types.type_for_hir_type(return_param.ty) else {
            return;
        };
        // Non-primitive return shapes can currently make transitive type-relation queries grow
        // too much; keep the first call-result slice to primitive signatures.
        if matches!(self.types[return_ty], Type::Primitive(_)) {
            self.intern_type_equality(TypeSubject::Expr(call), TypeSubject::Type(return_ty));
        }

        let arg_param_tys = args
            .iter()
            .zip(input_params)
            .filter_map(|(arg, param)| Some((*arg, self.types.type_for_hir_type(param.ty)?)))
            .collect::<Vec<_>>();
        if arg_param_tys.len() != args.len() {
            return;
        }
        for (arg, param_ty) in arg_param_tys {
            self.intern_type_equality(TypeSubject::Expr(arg), TypeSubject::Type(param_ty));
        }
    }

    fn fn_signature_for_def(&self, def: name::DefId) -> Option<hir::SignatureId> {
        self.hir.items().iter().find_map(|item| {
            if item.def != Some(def) {
                return None;
            }
            let hir::ItemKind::Fn { signature, .. } = item.kind else {
                return None;
            };
            Some(signature)
        })
    }

    fn collect_fn_return_facts(&mut self) {
        for item in self.hir.items() {
            let hir::ItemKind::Fn {
                signature, block, ..
            } = item.kind
            else {
                continue;
            };
            let Some(return_param) = self.hir[signature].params.first() else {
                continue;
            };
            let Some(return_ty) = self.types.type_for_hir_type(return_param.ty) else {
                continue;
            };
            self.collect_tail_return_fact(block, return_ty);
            self.collect_return_expr_facts_in_block(block, return_ty);
        }
    }

    fn collect_tail_return_fact(&mut self, block: hir::BlockId, return_ty: TypeId) {
        let Some(tail_expr) = self.hir.lowered_blocks()[block].tail_expr else {
            return;
        };
        self.intern_type_equality(TypeSubject::Expr(tail_expr), TypeSubject::Type(return_ty));
    }

    fn collect_return_expr_facts_in_block(&mut self, block: hir::BlockId, return_ty: TypeId) {
        let exprs = self.hir.lowered_blocks()[block]
            .stmts
            .iter()
            .filter_map(|stmt| match stmt {
                hir::lower::Stmt::Expr(expr) => Some(*expr),
                hir::lower::Stmt::Local(local) => local.init,
                hir::lower::Stmt::Item(_) => None,
            })
            .collect::<Vec<_>>();
        for expr in exprs {
            self.collect_return_expr_facts_in_expr(expr, return_ty);
        }
    }

    fn collect_return_expr_facts_in_expr(&mut self, expr: hir::ExprId, return_ty: TypeId) {
        match &self.hir[expr].kind {
            hir::ExprKind::Return { expr: Some(value) } => {
                let value = *value;
                self.intern_type_equality(TypeSubject::Expr(value), TypeSubject::Type(return_ty));
                self.collect_return_expr_facts_in_expr(value, return_ty);
            }
            hir::ExprKind::Return { expr: None }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::Path(_) => {}
            hir::ExprKind::Array { elems } | hir::ExprKind::Tuple { elems } => {
                let elems = elems.clone();
                for elem in elems {
                    self.collect_return_expr_facts_in_expr(elem, return_ty);
                }
            }
            hir::ExprKind::Assign { left, right } | hir::ExprKind::Binary { left, right } => {
                let (left, right) = (*left, *right);
                self.collect_return_expr_facts_in_expr(left, return_ty);
                self.collect_return_expr_facts_in_expr(right, return_ty);
            }
            hir::ExprKind::Block { block } => {
                self.collect_return_expr_facts_in_block(*block, return_ty);
            }
            hir::ExprKind::Call { func, args } => {
                let func = *func;
                let args = args.clone();
                self.collect_return_expr_facts_in_expr(func, return_ty);
                for arg in args {
                    self.collect_return_expr_facts_in_expr(arg, return_ty);
                }
            }
            hir::ExprKind::Cast { expr, .. }
            | hir::ExprKind::Field { base: expr, .. }
            | hir::ExprKind::Paren { expr }
            | hir::ExprKind::Reference { expr, .. }
            | hir::ExprKind::Repeat { expr, .. }
            | hir::ExprKind::Unary { expr } => {
                self.collect_return_expr_facts_in_expr(*expr, return_ty);
            }
            hir::ExprKind::Index { expr, index } => {
                let (expr, index) = (*expr, *index);
                self.collect_return_expr_facts_in_expr(expr, return_ty);
                self.collect_return_expr_facts_in_expr(index, return_ty);
            }
            hir::ExprKind::MethodCall { receiver, args, .. } => {
                let receiver = *receiver;
                let args = args.clone();
                self.collect_return_expr_facts_in_expr(receiver, return_ty);
                for arg in args {
                    self.collect_return_expr_facts_in_expr(arg, return_ty);
                }
            }
            hir::ExprKind::Struct { fields, rest, .. } => {
                let fields = fields.clone();
                let rest = *rest;
                for field in fields {
                    self.collect_return_expr_facts_in_expr(field.expr, return_ty);
                }
                if let Some(rest) = rest {
                    self.collect_return_expr_facts_in_expr(rest, return_ty);
                }
            }
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
            hir::PatKind::Tuple { elems } => {
                let pat_elems = elems.clone();
                let Type::Tuple { elems: ty_elems } = &self.types[ty] else {
                    return;
                };
                if pat_elems.len() != ty_elems.len() {
                    return;
                }
                let ty_elems = ty_elems.clone();
                for (pat, ty) in pat_elems.into_iter().zip(ty_elems) {
                    self.bind_pat_to_type(pat, ty);
                }
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
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
