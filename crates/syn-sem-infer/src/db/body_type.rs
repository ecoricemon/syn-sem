//! Body-local type facts and resolved type mappings for inference.

use super::infer_types::InferTypes;
use crate::{
    BodyBlockFact, BodyLocalFact, PrimitiveType, ResolvedTypeFact, Type, TypeEqualFact, TypeId,
    TypeSubject,
};
use syn_sem_common::Map;
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb, ResolveResult};

/// Body-local type facts owned by inference.
#[derive(Debug, Default)]
pub(crate) struct BodyTypeDb {
    /// Lowered block facts collected from HIR body lowering.
    pub(crate) blocks: Vec<BodyBlockFact>,
    /// Lowered local facts collected from HIR body lowering.
    pub(crate) locals: Vec<BodyLocalFact>,
    /// Body-local type equality facts.
    pub(crate) equalities: Vec<TypeEqualFact>,
    /// Concrete body-local type resolutions derived from equality facts.
    pub(crate) resolved: Vec<ResolvedTypeFact>,
    /// Resolved concrete types linked to HIR expression occurrences.
    pub(crate) expr_types: Map<hir::ExprId, TypeId>,
    /// Resolved concrete types linked to definitions.
    pub(crate) def_types: Map<DefId, TypeId>,
}

impl BodyTypeDb {
    /// Returns the resolved concrete type linked to a HIR expression occurrence.
    pub(crate) fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.expr_types.get(&hir_expr).copied()
    }

    /// Returns the resolved concrete type linked to a definition.
    pub(crate) fn type_for_def(&self, def: DefId) -> Option<TypeId> {
        self.def_types.get(&def).copied()
    }

    /// Records a body-local type equality fact.
    pub(crate) fn push_type_equal(&mut self, left: TypeSubject, right: TypeSubject) {
        let fact = TypeEqualFact { left, right };
        if !self.equalities.contains(&fact) {
            self.equalities.push(fact);
        }
    }

    /// Records resolved concrete types derived from equality facts.
    pub(crate) fn extend_resolved(&mut self, resolved: Vec<ResolvedTypeFact>) {
        for fact in &resolved {
            match fact.subject {
                TypeSubject::Def(def) => {
                    self.def_types.entry(def).or_insert(fact.ty);
                }
                TypeSubject::Expr(expr) => {
                    self.expr_types.entry(expr).or_insert(fact.ty);
                }
                TypeSubject::Type(_) => {}
            }
        }
        self.resolved.extend(resolved);
    }

    /// Returns lowered block facts collected from HIR body lowering.
    #[cfg(test)]
    pub(crate) fn blocks(&self) -> &[BodyBlockFact] {
        &self.blocks
    }

    /// Returns lowered local facts collected from HIR body lowering.
    #[cfg(test)]
    pub(crate) fn locals(&self) -> &[BodyLocalFact] {
        &self.locals
    }

    /// Returns body-local type equality facts.
    #[cfg(test)]
    pub(crate) fn equalities(&self) -> &[TypeEqualFact] {
        &self.equalities
    }

    /// Returns concrete body-local type resolutions derived from equality facts.
    #[cfg(test)]
    pub(crate) fn resolved(&self) -> &[ResolvedTypeFact] {
        &self.resolved
    }
}

pub(super) struct BodyTypeCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
    body_types: BodyTypeDb,
}

impl<'a, 'cx> BodyTypeCollector<'a, 'cx> {
    pub(super) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> BodyTypeDb {
        Self {
            hir,
            names,
            types,
            body_types: BodyTypeDb::default(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> BodyTypeDb {
        self.collect_signature_facts();
        self.collect_expr_facts();
        self.collect_body_facts();
        self.body_types
    }

    fn collect_signature_facts(&mut self) {
        for signature in self.hir.signatures() {
            for param in signature.params.iter().skip(1) {
                self.collect_pat_type_facts(param.pat, param.ty);
            }
        }
    }

    fn collect_expr_facts(&mut self) {
        for expr in self.hir.exprs() {
            match &expr.kind {
                hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                    if let Some(tail_expr) = self.hir.body()[*block].tail_expr {
                        self.push_type_equal(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Expr(tail_expr),
                        );
                    }
                }
                hir::ExprKind::Cast { ty, .. } => {
                    self.push_type_equal(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Type(
                            self.type_for_hir_type(*ty)
                                .expect("HIR types are lowered before body facts"),
                        ),
                    );
                }
                hir::ExprKind::Lit(lit) => {
                    if let Some(ty) = self.lit_type(lit) {
                        self.push_type_equal(TypeSubject::Expr(expr.id), TypeSubject::Type(ty));
                    }
                }
                hir::ExprKind::Paren { expr: inner } => {
                    self.push_type_equal(TypeSubject::Expr(expr.id), TypeSubject::Expr(*inner));
                }
                hir::ExprKind::Path(path) => {
                    if let Some(def) = resolve_value_path(self.names, expr.scope, &path.segments) {
                        self.push_type_equal(TypeSubject::Expr(expr.id), TypeSubject::Def(def));
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
            let Some(tail_expr) = self.hir.body()[block].tail_expr else {
                continue;
            };
            let Some(return_param) = self.hir[signature].params.first() else {
                continue;
            };
            let Some(return_ty) = self.type_for_hir_type(return_param.ty) else {
                continue;
            };
            self.push_type_equal(TypeSubject::Expr(tail_expr), TypeSubject::Type(return_ty));
        }
    }

    fn collect_body_facts(&mut self) {
        for block in self.hir.body().blocks() {
            self.body_types.blocks.push(BodyBlockFact {
                block: block.block,
                tail_expr: block.tail_expr,
            });

            for stmt in &block.stmts {
                let hir::lower::Stmt::Local(local) = stmt else {
                    continue;
                };
                self.body_types.locals.push(BodyLocalFact {
                    block: block.block,
                    local: local.local,
                    bindings: local.bindings.clone(),
                    init: local.init,
                });
                self.collect_local_facts(local);
            }
        }
    }

    fn collect_local_facts(&mut self, local: &hir::lower::Local) {
        if let Some(init) = local.init {
            self.collect_pat_expr_facts(Some(local.pat), init);
        }
    }

    fn collect_pat_type_facts(&mut self, pat: Option<hir::PatId>, ty: hir::TypeId) {
        let Some(pat) = pat else {
            return;
        };
        let Some(ty) = self.type_for_hir_type(ty) else {
            return;
        };
        self.bind_pat_to_type(pat, ty);
    }

    fn bind_pat_to_type(&mut self, pat: hir::PatId, ty: TypeId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.push_type_equal(TypeSubject::Def(*def), TypeSubject::Type(ty));
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

    fn collect_pat_expr_facts(&mut self, pat: Option<hir::PatId>, expr: hir::ExprId) {
        let Some(pat) = pat else {
            return;
        };
        self.bind_pat_to_expr(pat, expr);
    }

    fn bind_pat_to_expr(&mut self, pat: hir::PatId, expr: hir::ExprId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.push_type_equal(TypeSubject::Def(*def), TypeSubject::Expr(expr));
            }
            hir::PatKind::Reference { pat, .. } => self.bind_pat_to_expr(*pat, expr),
            hir::PatKind::Type { pat, ty } => {
                let Some(ty) = self.type_for_hir_type(*ty) else {
                    return;
                };
                self.bind_pat_to_type(*pat, ty);
                self.push_type_equal(TypeSubject::Expr(expr), TypeSubject::Type(ty));
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    fn type_for_hir_type(&self, hir_type: hir::TypeId) -> Option<TypeId> {
        self.types.type_for_hir_type(hir_type)
    }

    fn lit_type(&mut self, lit: &hir::Lit<'cx>) -> Option<TypeId> {
        match lit {
            hir::Lit::Bool(_) => Some(self.intern_type(Type::Primitive(PrimitiveType::Bool))),
            hir::Lit::Int(_) | hir::Lit::Float(_) => None,
        }
    }

    fn intern_type(&mut self, ty: Type<'cx>) -> TypeId {
        if let Some(index) = self.types.iter().position(|existing| existing == &ty) {
            return TypeId::new(index);
        }
        self.types.push_type(ty)
    }

    fn push_type_equal(&mut self, left: TypeSubject, right: TypeSubject) {
        self.body_types.push_type_equal(left, right);
    }
}

fn resolve_value_path<'cx>(
    names: &NameDb<'cx>,
    scope: Option<syn_sem_name::ScopeId>,
    path: &[hir::PathSegment<'cx>],
) -> Option<syn_sem_name::DefId> {
    let scope = scope?;
    match names.resolve_value_path(scope, path.iter().map(|segment| segment.name)) {
        ResolveResult::Found(def) => Some(def),
        ResolveResult::Ambiguous(_) | ResolveResult::NotFound => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::InferDb;
    use syn_sem_ast as ast;
    use syn_sem_ast::SyntaxCx;
    use syn_sem_common::CommonCx;
    use syn_sem_hir::{Hir, HirBuilder, ItemKind};
    use syn_sem_name::{collect::NameCollector, DefKind, NameDb};

    fn analyze<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (NameDb<'cx>, Hir<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("body_infer_test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text)
            .expect("test input should parse");
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameCollector::new([ast::SourceInput { file_path, file }])
            .collect(file_path)
            .expect("name collection should succeed");
        let hir = HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, &names);
        (names, hir, infer)
    }

    fn function_block(hir: &Hir<'_>, name: &str) -> hir::BlockId {
        let item = hir
            .items()
            .iter()
            .find(|item| {
                item.name
                    .is_some_and(|item_name| item_name.as_ref() == name)
            })
            .unwrap_or_else(|| panic!("expected function `{name}`"));
        let ItemKind::Fn { block, .. } = item.kind else {
            panic!("expected function item");
        };
        block
    }

    #[test]
    fn consumes_hir_lowered_body_facts() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(pair: (usize, usize)) -> usize {
                let (a, b) = pair;
                a
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let block_fact = infer
            .body_types
            .blocks()
            .iter()
            .find(|fact| fact.block == block)
            .expect("expected function block fact");
        assert_eq!(block_fact.tail_expr, hir.body()[block].tail_expr);

        let local_fact = infer
            .body_types
            .locals()
            .iter()
            .find(|fact| fact.block == block)
            .expect("expected local fact");
        assert_eq!(local_fact.bindings.len(), 2);
        assert!(local_fact.init.is_some());
        assert!(local_fact
            .bindings
            .iter()
            .all(|def| names[*def].kind == DefKind::Local));
    }

    #[test]
    fn resolves_simple_body_types_through_logic_equalities() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) -> usize {
                let y = x;
                y
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local), hir::lower::Stmt::Expr(tail)] =
            hir.body()[block].stmts.as_slice()
        else {
            panic!("expected local statement followed by tail expression");
        };
        let init = local.init.expect("local should have initializer");
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert!(infer.body_types.equalities().iter().any(|fact| {
            fact.left == TypeSubject::Def(local_def) && fact.right == TypeSubject::Expr(init)
        }));
        assert_usize(&infer, infer.type_for_def(local_def));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        let tail_ty = infer.type_for_hir_expr(*tail);
        assert_usize(&infer, tail_ty);
        assert!(infer
            .body_types
            .resolved()
            .iter()
            .any(|fact| { fact.subject == TypeSubject::Expr(*tail) && Some(fact.ty) == tail_ty }));
        assert!(matches!(names[local_def].kind, DefKind::Local));
    }

    fn assert_usize(infer: &InferDb<'_>, ty: Option<TypeId>) {
        let ty = ty.expect("expected a resolved type");
        assert_eq!(infer[ty], Type::Primitive(PrimitiveType::Usize));
    }
}
