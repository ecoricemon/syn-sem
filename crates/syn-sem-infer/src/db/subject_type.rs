//! Subject type relations and resolved type mappings for inference.

use super::infer_types::InferTypes;
use crate::{PrimitiveType, Type, TypeId};
use syn_sem_common::Map;
use syn_sem_hir as hir;
use syn_sem_name as name;

pub(crate) struct SubjectTypeCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a name::NameDb<'cx>,
    types: &'a mut InferTypes<'cx>,
    subject_equalities: Vec<TypeEqualFact>,
}

impl<'a, 'cx> SubjectTypeCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a name::NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> SubjectTypeDb {
        Self {
            hir,
            names,
            types,
            subject_equalities: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> SubjectTypeDb {
        self.collect_signature_facts();
        self.collect_expr_facts();
        self.collect_lowered_block_facts();

        SubjectTypeDb {
            equalities: self.subject_equalities,
            resolved: Vec::new(),
            expr_types: Map::default(),
            def_types: Map::default(),
        }
    }

    fn collect_signature_facts(&mut self) {
        for signature in self.hir.signatures() {
            for param in signature.params.iter().skip(1) {
                if let Some(pat) = param.pat {
                    self.collect_pat_type_facts(pat, param.ty_id);
                }
            }
        }
    }

    fn collect_expr_facts(&mut self) {
        for expr in self.hir.exprs() {
            match &expr.kind {
                hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                    if let Some(tail_expr) = self.hir.lowered_blocks()[*block].tail_expr {
                        self.intern_type_equal(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Expr(tail_expr),
                        );
                    }
                }
                hir::ExprKind::Cast { ty_id, .. } => {
                    self.intern_type_equal(
                        TypeSubject::Expr(expr.id),
                        TypeSubject::Type(
                            self.types
                                .type_for_hir_type(*ty_id)
                                .expect("HIR types are lowered before subject type relations"),
                        ),
                    );
                }
                hir::ExprKind::Lit(lit) => {
                    if let Some(ty_id) = self.lit_type(lit) {
                        self.intern_type_equal(
                            TypeSubject::Expr(expr.id),
                            TypeSubject::Type(ty_id),
                        );
                    }
                }
                hir::ExprKind::Paren { expr: inner } => {
                    self.intern_type_equal(TypeSubject::Expr(expr.id), TypeSubject::Expr(*inner));
                }
                hir::ExprKind::Path(path) => {
                    if let Some(def) =
                        Self::resolve_value_path(self.names, expr.scope, &path.segments)
                    {
                        self.intern_type_equal(TypeSubject::Expr(expr.id), TypeSubject::Def(def));
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
            let Some(return_ty_id) = self.types.type_for_hir_type(return_param.ty_id) else {
                continue;
            };
            self.intern_type_equal(
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
        if let Some(ty_id) = self.types.type_for_hir_type(hir_ty_id) {
            self.bind_pat_to_type(pat, ty_id);
        }
    }

    fn bind_pat_to_type(&mut self, pat: hir::PatId, ty_id: TypeId) {
        match &self.hir[pat].kind {
            hir::PatKind::Ident { def: Some(def), .. } => {
                self.intern_type_equal(TypeSubject::Def(*def), TypeSubject::Type(ty_id));
            }
            hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
                self.bind_pat_to_type(*pat, ty_id);
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
                self.intern_type_equal(TypeSubject::Def(*def), TypeSubject::Expr(expr));
            }
            hir::PatKind::Reference { pat, .. } => self.bind_pat_to_expr(*pat, expr),
            hir::PatKind::Type { pat, ty_id } => {
                let Some(ty_id) = self.types.type_for_hir_type(*ty_id) else {
                    return;
                };
                self.bind_pat_to_type(*pat, ty_id);
                self.intern_type_equal(TypeSubject::Expr(expr), TypeSubject::Type(ty_id));
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

    fn intern_type_equal(&mut self, left: TypeSubject, right: TypeSubject) {
        let fact = TypeEqualFact { left, right };
        if !self.subject_equalities.contains(&fact) {
            self.subject_equalities.push(fact);
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

/// Type relations between inference subjects.
#[derive(Debug, Default)]
pub(crate) struct SubjectTypeDb {
    /// Equality relations between inference subjects.
    pub(crate) equalities: Vec<TypeEqualFact>,
    /// Type resolutions derived from equality relations.
    pub(crate) resolved: Vec<ResolvedTypeFact>,
    /// Resolved types linked to HIR expression occurrences.
    pub(crate) expr_types: Map<hir::ExprId, TypeId>,
    /// Resolved types linked to definitions.
    pub(crate) def_types: Map<name::DefId, TypeId>,
}

impl SubjectTypeDb {
    /// Returns the resolved type linked to a HIR expression occurrence.
    pub(crate) fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.expr_types.get(&hir_expr).copied()
    }

    /// Returns the resolved type linked to a definition.
    pub(crate) fn type_for_def(&self, def: name::DefId) -> Option<TypeId> {
        self.def_types.get(&def).copied()
    }

    /// Records resolved subject types derived from equality relations.
    pub(crate) fn extend_resolved(&mut self, resolved: Vec<ResolvedTypeFact>) {
        for fact in &resolved {
            match fact.subject {
                TypeSubject::Def(def) => {
                    self.def_types.entry(def).or_insert(fact.ty_id);
                }
                TypeSubject::Expr(expr) => {
                    self.expr_types.entry(expr).or_insert(fact.ty_id);
                }
                TypeSubject::Type(_) => {}
            }
        }
        self.resolved.extend(resolved);
    }

    /// Returns equality relations between inference subjects.
    #[cfg(test)]
    pub(crate) fn equalities(&self) -> &[TypeEqualFact] {
        &self.equalities
    }

    /// Returns type resolutions derived from equality relations.
    #[cfg(test)]
    pub(crate) fn resolved(&self) -> &[ResolvedTypeFact] {
        &self.resolved
    }
}

/// Equality relation between two inference subjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeEqualFact {
    /// Left side of the equality edge.
    pub(crate) left: TypeSubject,
    /// Right side of the equality edge.
    pub(crate) right: TypeSubject,
}

/// Resolved type found for an inference subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedTypeFact {
    /// Subject being resolved.
    pub(crate) subject: TypeSubject,
    /// Inference type selected for the subject through equality edges.
    pub(crate) ty_id: TypeId,
}

/// Subject whose type can participate in subject type inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TypeSubject {
    /// A definition such as a parameter or local binding.
    Def(name::DefId),
    /// A HIR expression occurrence.
    Expr(hir::ExprId),
    /// An inference type.
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
        let file_path = ccx.intern("subject_type_infer_test.rs");
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
    fn resolves_simple_subject_types_through_logic_equalities() {
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
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected local statement followed by tail expression");
        };
        let init = local.init.expect("local should have initializer");
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert!(infer.subject_types.equalities().iter().any(|fact| {
            fact.left == TypeSubject::Def(local_def) && fact.right == TypeSubject::Expr(init)
        }));
        assert_usize(&infer, infer.type_for_def(local_def));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        let tail_ty_id = infer.type_for_hir_expr(*tail);
        assert_usize(&infer, tail_ty_id);
        assert!(infer.subject_types.resolved().iter().any(|fact| {
            fact.subject == TypeSubject::Expr(*tail) && Some(fact.ty_id) == tail_ty_id
        }));
        assert!(matches!(names[local_def].kind, DefKind::Local));
    }

    #[test]
    fn keeps_unconstrained_numeric_literals_abstract() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a = 1;
                let b = 1.0;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(int_local), hir::lower::Stmt::Local(float_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let int_init = int_local.init.expect("int local should have initializer");
        let int_def = int_local
            .bindings
            .first()
            .copied()
            .expect("int local should introduce a binding");
        let float_init = float_local
            .init
            .expect("float local should have initializer");
        let float_def = float_local
            .bindings
            .first()
            .copied()
            .expect("float local should introduce a binding");

        assert_primitive(
            &infer,
            infer.type_for_def(int_def),
            PrimitiveType::AbstractInt,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(int_init),
            PrimitiveType::AbstractInt,
        );
        assert_primitive(
            &infer,
            infer.type_for_def(float_def),
            PrimitiveType::AbstractFloat,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(float_init),
            PrimitiveType::AbstractFloat,
        );
    }

    #[test]
    fn refines_abstract_numeric_literals_to_concrete_primitives() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() -> i32 {
                let a: i32 = 1;
                a
            }

            fn g() -> f64 {
                let b: f64 = 1.0;
                b
            }
            "#,
        );

        assert_typed_numeric_local(&hir, &infer, "f", PrimitiveType::I32);
        assert_typed_numeric_local(&hir, &infer, "g", PrimitiveType::F64);
    }

    #[test]
    fn keeps_fresh_abstract_numeric_literals_separate() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a: i32 = 1;
                let b = 2;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(typed_local), hir::lower::Stmt::Local(untyped_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let typed_def = typed_local
            .bindings
            .first()
            .copied()
            .expect("typed local should introduce a binding");
        let untyped_def = untyped_local
            .bindings
            .first()
            .copied()
            .expect("untyped local should introduce a binding");

        assert_primitive(&infer, infer.type_for_def(typed_def), PrimitiveType::I32);
        assert_primitive(
            &infer,
            infer.type_for_def(untyped_def),
            PrimitiveType::AbstractInt,
        );
    }

    #[test]
    fn rejects_incompatible_abstract_numeric_resolution() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a: bool = 1;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected one local statement");
        };
        let init = local.init.expect("local should have initializer");
        let def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce a binding");

        assert_eq!(infer.type_for_def(def), None);
        assert_eq!(infer.type_for_hir_expr(init), None);
    }

    fn assert_typed_numeric_local(
        hir: &Hir<'_>,
        infer: &InferDb<'_>,
        name: &str,
        expected: PrimitiveType,
    ) {
        let block = function_block(hir, name);
        let [hir::lower::Stmt::Local(local), hir::lower::Stmt::Expr(tail)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected local statement followed by tail expression");
        };
        let init = local.init.expect("local should have initializer");
        let def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce a binding");

        assert_primitive(infer, infer.type_for_def(def), expected);
        assert_primitive(infer, infer.type_for_hir_expr(init), expected);
        assert_primitive(infer, infer.type_for_hir_expr(*tail), expected);
    }

    fn assert_usize(infer: &InferDb<'_>, ty_id: Option<TypeId>) {
        assert_primitive(infer, ty_id, PrimitiveType::Usize);
    }

    fn assert_primitive(infer: &InferDb<'_>, ty_id: Option<TypeId>, expected: PrimitiveType) {
        let ty_id = ty_id.expect("expected a resolved type");
        assert_eq!(infer[ty_id], Type::Primitive(expected));
    }
}
