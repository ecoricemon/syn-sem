//! Body-local type facts and resolved type mappings for inference.

use super::infer_types::InferTypes;
use crate::{PrimitiveType, Type, TypeId};
use syn_sem_common::Map;
use syn_sem_hir as hir;
use syn_sem_name as name;

pub(crate) struct BodyTypeCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a name::NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
    body_equalities: Vec<TypeEqualFact>,
}

impl<'a, 'cx> BodyTypeCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a name::NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> BodyTypeDb {
        Self {
            hir,
            names,
            types,
            body_equalities: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> BodyTypeDb {
        self.collect_signature_facts();
        self.collect_expr_facts();
        self.collect_body_facts();

        BodyTypeDb {
            equalities: self.body_equalities,
            resolved: Vec::new(),
            expr_types: Map::default(),
            def_types: Map::default(),
        }
    }

    fn collect_signature_facts(&mut self) {
        for signature in self.hir.signatures() {
            for param in signature.params.iter().skip(1) {
                self.collect_pat_type_facts(param.pat, param.tid);
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
                hir::ExprKind::Cast { tid, .. } => {
                    self.push_type_equal(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Type(
                            self.type_for_hir_type(*tid)
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
                    if let Some(def) =
                        Self::resolve_value_path(self.names, expr.scope, &path.segments)
                    {
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
            let Some(return_tid) = self.type_for_hir_type(return_param.tid) else {
                continue;
            };
            self.push_type_equal(TypeSubject::Expr(tail_expr), TypeSubject::Type(return_tid));
        }
    }

    fn collect_body_facts(&mut self) {
        for block in self.hir.body().blocks() {
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
            self.collect_pat_expr_facts(Some(local.pat), init);
        }
    }

    fn collect_pat_type_facts(&mut self, pat: Option<hir::PatId>, hir_tid: hir::TypeId) {
        let Some(pat) = pat else {
            return;
        };
        let Some(tid) = self.type_for_hir_type(hir_tid) else {
            return;
        };
        self.bind_pat_to_type(pat, tid);
    }

    fn bind_pat_to_type(&mut self, pat: hir::PatId, tid: TypeId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.push_type_equal(TypeSubject::Def(*def), TypeSubject::Type(tid));
            }
            hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
                self.bind_pat_to_type(*pat, tid);
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
            hir::PatKind::Type { pat, tid } => {
                let Some(tid) = self.type_for_hir_type(*tid) else {
                    return;
                };
                self.bind_pat_to_type(*pat, tid);
                self.push_type_equal(TypeSubject::Expr(expr), TypeSubject::Type(tid));
            }
            hir::PatKind::Ident { def: None, .. }
            | hir::PatKind::Path(_)
            | hir::PatKind::Struct { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    fn type_for_hir_type(&self, hir_tid: hir::TypeId) -> Option<TypeId> {
        self.types.type_for_hir_type(hir_tid)
    }

    fn lit_type(&mut self, lit: &hir::Lit<'cx>) -> Option<TypeId> {
        match lit {
            hir::Lit::Bool(_) => Some(self.types.intern_type(Type::Primitive(PrimitiveType::Bool))),
            hir::Lit::Int(_) | hir::Lit::Float(_) => None,
        }
    }

    fn push_type_equal(&mut self, left: TypeSubject, right: TypeSubject) {
        let fact = TypeEqualFact { left, right };
        if !self.body_equalities.contains(&fact) {
            self.body_equalities.push(fact);
        }
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

/// Body-local type facts owned by inference.
#[derive(Debug, Default)]
pub(crate) struct BodyTypeDb {
    /// Body-local type equality facts.
    pub(crate) equalities: Vec<TypeEqualFact>,
    /// Concrete body-local type resolutions derived from equality facts.
    pub(crate) resolved: Vec<ResolvedTypeFact>,
    /// Resolved concrete types linked to HIR expression occurrences.
    pub(crate) expr_types: Map<hir::ExprId, TypeId>,
    /// Resolved concrete types linked to definitions.
    pub(crate) def_types: Map<name::DefId, TypeId>,
}

impl BodyTypeDb {
    /// Returns the resolved concrete type linked to a HIR expression occurrence.
    pub(crate) fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.expr_types.get(&hir_expr).copied()
    }

    /// Returns the resolved concrete type linked to a definition.
    pub(crate) fn type_for_def(&self, def: name::DefId) -> Option<TypeId> {
        self.def_types.get(&def).copied()
    }

    /// Records resolved concrete types derived from equality facts.
    pub(crate) fn extend_resolved(&mut self, resolved: Vec<ResolvedTypeFact>) {
        for fact in &resolved {
            match fact.subject {
                TypeSubject::Def(def) => {
                    self.def_types.entry(def).or_insert(fact.tid);
                }
                TypeSubject::Expr(expr) => {
                    self.expr_types.entry(expr).or_insert(fact.tid);
                }
                TypeSubject::Type(_) => {}
            }
        }
        self.resolved.extend(resolved);
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

/// Body-local type equality edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeEqualFact {
    /// Left side of the equality edge.
    pub(crate) left: TypeSubject,
    /// Right side of the equality edge.
    pub(crate) right: TypeSubject,
}

/// Resolved concrete type found for a body-local subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedTypeFact {
    /// Subject being resolved.
    pub(crate) subject: TypeSubject,
    /// Concrete inference type reachable from the subject through equality edges.
    pub(crate) tid: TypeId,
}

/// Subject whose type can participate in body-local type equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TypeSubject {
    /// A definition such as a parameter or local binding.
    Def(name::DefId),
    /// A HIR expression occurrence.
    Expr(hir::ExprId),
    /// A concrete inference type.
    Type(TypeId),
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
        let tail_tid = infer.type_for_hir_expr(*tail);
        assert_usize(&infer, tail_tid);
        assert!(infer.body_types.resolved().iter().any(|fact| {
            fact.subject == TypeSubject::Expr(*tail) && Some(fact.tid) == tail_tid
        }));
        assert!(matches!(names[local_def].kind, DefKind::Local));
    }

    fn assert_usize(infer: &InferDb<'_>, tid: Option<TypeId>) {
        let tid = tid.expect("expected a resolved type");
        assert_eq!(infer[tid], Type::Primitive(PrimitiveType::Usize));
    }
}
