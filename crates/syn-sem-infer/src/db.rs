//! Inference database orchestration and public query surface.
//!
//! `InferDbBuilder` wires the source-shaped phases in order: program facts, HIR type lowering,
//! type relation collection, projection obligation collection, projection normalization, and
//! fixed-point type relation/expression type resolution. The resulting [`InferDb`] exposes
//! focused query methods for upper semantic phases.

use crate::{
    ExprTypeDeriver, GenericArg, ImplAssocType, ImplAssocTypeCollector, InferTypes, LogicSession,
    PatTypeDeriver, Path, PathSegment, PathType, PathTypeResolution, ProjectionCollector,
    ProjectionDb, ProjectionNormalizationResult, ProjectionNormalizer, ProjectionType, QSelf,
    TraitBound, TraitBoundCollector, Type, TypeId, TypeLowering, TypeParamBound,
    TypeRelationCollector, TypeRelationDb, TypeRelationResolver,
};
use std::convert::TryFrom;
use std::ops::Index;
use syn_sem_common::{CommonCx, Map, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb};

pub(crate) struct InferDbBuilder<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
}

impl<'a, 'cx> InferDbBuilder<'a, 'cx> {
    pub(crate) fn new(hir: &'a hir::Hir<'cx>, names: &'a NameDb<'cx>) -> Self {
        Self { hir, names }
    }

    pub(crate) fn build(self) -> InferDb<'cx> {
        let mut types = InferTypes::default();
        let trait_bounds = TraitBoundCollector::collect(self.hir, self.names, &mut types);
        TypeLowering::lower_hir_types(self.hir, self.names, &mut types);
        let impl_assoc_types = ImplAssocTypeCollector::collect(self.hir, self.names, &types);
        let type_relations = TypeRelationCollector::collect(self.hir, self.names, &mut types);
        let projections = ProjectionCollector::collect(&types);

        InferDb {
            types,
            projections,
            type_relations,
            trait_bounds,
            impl_assoc_types,
            recursive_normalizations: Map::default(),
        }
    }
}

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    pub(crate) types: InferTypes<'cx>,
    pub(crate) projections: ProjectionDb,
    pub(crate) type_relations: TypeRelationDb,
    pub(crate) trait_bounds: Vec<TraitBound>,
    pub(crate) impl_assoc_types: Vec<ImplAssocType>,
    pub(crate) recursive_normalizations: Map<TypeId, TypeId>,
}

impl<'cx> InferDb<'cx> {
    /// Builds inference type facts from HIR, name-resolution data, and optional constant facts.
    ///
    /// Analysis runs in source-shaped phases:
    ///
    /// * collect trait bounds, HIR type occurrences, impl associated types, type
    ///   relation equality facts, and projection obligations;
    /// * normalize associated type projections through trait matches and impl associated types;
    /// * iterate type relation resolution with expression result and pattern binding fact derivation;
    /// * expose the resolved expression and definition type lookups.
    ///
    /// Expression result inference is still narrow and currently derives only the expression forms
    /// represented by the expression type derivation phase.
    pub fn analyze(
        ccx: &'cx CommonCx,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        const_facts: &InferConstFacts,
    ) -> Self {
        let mut db = InferDbBuilder::new(hir, names).build();
        let mut logic_session = LogicSession::default();

        ProjectionNormalizer::new(
            &mut db.projections,
            &mut db.types,
            ccx,
            &db.trait_bounds,
            &db.impl_assoc_types,
            names,
            const_facts,
            &mut logic_session,
        )
        .normalize();
        db.resolve_type_relations(ccx, hir, names, const_facts, &mut logic_session);

        db
    }

    fn resolve_type_relations(
        &mut self,
        ccx: &'cx CommonCx,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        const_facts: &InferConstFacts,
        logic_session: &mut LogicSession<'cx>,
    ) {
        loop {
            self.type_relations.clear_resolved();
            TypeRelationResolver::new(&mut self.type_relations, &self.types).resolve();
            let obligation_count = self.projections.obligations.len();
            let expr_changed = ExprTypeDeriver::new(
                ccx,
                hir,
                names,
                &mut self.projections,
                &mut self.type_relations,
                &mut self.types,
                const_facts,
            )
            .derive();
            if self.projections.obligations.len() != obligation_count {
                ProjectionNormalizer::new(
                    &mut self.projections,
                    &mut self.types,
                    ccx,
                    &self.trait_bounds,
                    &self.impl_assoc_types,
                    names,
                    const_facts,
                    logic_session,
                )
                .normalize();
            }
            let changed = expr_changed
                | PatTypeDeriver::new(hir, &mut self.type_relations, &self.types).derive();
            if !changed {
                break;
            }
        }
    }

    /// Returns the inference type linked to a HIR type occurrence.
    pub fn type_for_hir_type(&self, hir_ty_id: hir::TypeId) -> Option<TypeId> {
        self.types.type_for_hir_type(hir_ty_id)
    }

    /// Returns the resolved concrete type linked to a HIR expression occurrence.
    pub fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.type_relations.type_for_hir_expr(hir_expr)
    }

    /// Returns the resolved concrete type linked to a definition, when type relation resolution found one.
    pub fn type_for_def(&self, def: DefId) -> Option<TypeId> {
        self.type_relations.type_for_def(def)
    }

    /// Returns the shallow normalized inference type linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered.
    #[cfg(test)]
    pub(crate) fn shallow_normalized_type_for_hir_type(
        &self,
        hir_ty_id: hir::TypeId,
    ) -> Option<TypeId> {
        self.type_for_hir_type(hir_ty_id)
            .map(|ty| self.shallow_normalized_type(ty))
    }

    /// Returns the unique normalized projection value linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered, is not a
    /// projection with a known normalization, or currently has multiple possible normalizations.
    #[cfg(test)]
    pub(crate) fn normalized_projection_type_for_hir_type(
        &self,
        hir_ty_id: hir::TypeId,
    ) -> Option<TypeId> {
        let ty = self.type_for_hir_type(hir_ty_id)?;
        self.normalized_projection_type(ty)
    }

    /// Returns the recursively normalized inference type linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered.
    pub fn normalized_type_for_hir_type(&mut self, hir_ty_id: hir::TypeId) -> Option<TypeId> {
        let ty = self.type_for_hir_type(hir_ty_id)?;
        Some(self.normalized_type(ty))
    }

    /// Returns the unique normalized value type for one associated type projection.
    ///
    /// Returns `None` when the projection has no known normalization or when multiple
    /// normalizations are currently possible.
    #[cfg(test)]
    pub(crate) fn normalized_projection_type(&self, projection: TypeId) -> Option<TypeId> {
        self.projections
            .normalized_type(projection, self.projection(projection).is_some())
    }

    /// Returns the normalization query result for one associated type projection.
    pub fn projection_normalization(&self, projection: TypeId) -> ProjectionNormalizationResult {
        self.projections
            .normalization(projection, self.projection(projection).is_some())
    }

    /// Returns the shallow normalized form of an inference type.
    ///
    /// This only normalizes the type itself when it is an associated type projection. It does not
    /// recursively rewrite nested type arguments.
    #[cfg(test)]
    pub(crate) fn shallow_normalized_type(&self, ty: TypeId) -> TypeId {
        if self.projection(ty).is_some() {
            if let Some(value_ty) = self.normalized_projection_type(ty) {
                return value_ty;
            }
        }
        ty
    }

    /// Returns the recursively normalized form of an inference type.
    ///
    /// This rewrites associated type projections in the type itself and in nested type positions
    /// for the currently supported type shapes.
    pub fn normalized_type(&mut self, ty: TypeId) -> TypeId {
        if let Some(normalized) = self.recursive_normalizations.get(&ty).copied() {
            return normalized;
        }
        let normalized = self.normalized_type_inner(ty, &mut Vec::new());
        self.recursive_normalizations.insert(ty, normalized);
        normalized
    }

    pub(crate) fn intern_type(&mut self, ty: Type<'cx>) -> TypeId {
        self.types.intern_type(ty)
    }

    /// Returns the path resolution for a path type.
    pub fn path_resolution(&self, ty: TypeId) -> Option<&PathTypeResolution> {
        self.types.path_resolution(ty)
    }

    /// Returns the nominal definition named by a type, when the type is a nominal path.
    pub fn nominal_def(&self, ty: TypeId) -> Option<DefId> {
        self.types.nominal_def(ty)
    }

    /// Returns the generic parameter definition named by a type, when the type is a generic path.
    pub fn generic_param_def(&self, ty: TypeId) -> Option<DefId> {
        let PathTypeResolution::GenericParam(def) = self.path_resolution(ty)? else {
            return None;
        };
        Some(*def)
    }

    /// Returns associated type projection metadata for a projection path type.
    pub fn projection(&self, ty: TypeId) -> Option<&ProjectionType> {
        self.projections.projection(self.path_resolution(ty))
    }

    fn normalized_type_inner(&mut self, ty: TypeId, active_ids: &mut Vec<TypeId>) -> TypeId {
        if !active_ids.push_unique(ty) {
            return ty;
        }

        match self.projection_normalization(ty) {
            ProjectionNormalizationResult::Known(value_ty) if value_ty != ty => {
                let normalized = self.normalized_type_inner(value_ty, active_ids);
                active_ids.pop();
                return normalized;
            }
            ProjectionNormalizationResult::Known(_)
            | ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => {}
        }

        let normalized = match self[ty].clone() {
            Type::Array { elem, len } => {
                let elem = self.normalized_type_inner(elem, active_ids);
                self.intern_changed_type(ty, Type::Array { elem, len })
            }
            Type::Infer | Type::Primitive(_) => ty,
            Type::Path(path) => {
                let (path, changed) = self.normalized_path_type(path, active_ids);
                if changed {
                    self.intern_type(Type::Path(path))
                } else {
                    ty
                }
            }
            Type::Reference { elem, is_mut } => {
                let elem = self.normalized_type_inner(elem, active_ids);
                self.intern_changed_type(ty, Type::Reference { elem, is_mut })
            }
            Type::Slice { elem } => {
                let elem = self.normalized_type_inner(elem, active_ids);
                self.intern_changed_type(ty, Type::Slice { elem })
            }
            Type::Tuple { elems } => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.normalized_type_inner(elem, active_ids))
                    .collect();
                self.intern_changed_type(ty, Type::Tuple { elems })
            }
        };

        active_ids.pop();
        normalized
    }

    fn intern_changed_type(&mut self, original_id: TypeId, ty: Type<'cx>) -> TypeId {
        if self[original_id] == ty {
            return original_id;
        }
        self.intern_type(ty)
    }

    fn normalized_path_type(
        &mut self,
        path: PathType<'cx>,
        active: &mut Vec<TypeId>,
    ) -> (PathType<'cx>, bool) {
        let mut changed = false;
        let qself = path.qself.map(|qself| {
            let self_ = self.normalized_type_inner(qself.self_, active);
            changed |= self_ != qself.self_;
            let trait_ = qself.trait_.map(|trait_| {
                let normalized = self.normalized_type_inner(trait_, active);
                changed |= normalized != trait_;
                normalized
            });
            QSelf { self_, trait_ }
        });
        let segments = path
            .path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| self.normalized_generic_argument(arg, active, &mut changed))
                    .collect();
                PathSegment {
                    name: segment.name,
                    args,
                }
            })
            .collect();

        (
            PathType {
                qself,
                path: Path { segments },
                resolution: path.resolution,
            },
            changed,
        )
    }

    fn normalized_generic_argument(
        &mut self,
        arg: GenericArg<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> GenericArg<'cx> {
        match arg {
            GenericArg::Type(ty) => {
                let normalized = self.normalized_type_inner(ty, active);
                *changed |= normalized != ty;
                GenericArg::Type(normalized)
            }
            GenericArg::AssocType { name, ty } => {
                let normalized = self.normalized_type_inner(ty, active);
                *changed |= normalized != ty;
                GenericArg::AssocType {
                    name,
                    ty: normalized,
                }
            }
            GenericArg::Const(arg) => GenericArg::Const(arg),
            GenericArg::AssocConst { name, value } => GenericArg::AssocConst { name, value },
            GenericArg::Constraint { name, bounds } => {
                let bounds = self.normalized_type_bounds(bounds, active, changed);
                GenericArg::Constraint { name, bounds }
            }
            GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn normalized_type_bounds(
        &mut self,
        bounds: Vec<TypeParamBound<'cx>>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> Vec<TypeParamBound<'cx>> {
        bounds
            .into_iter()
            .map(|bound| self.normalized_type_param_bound(bound, active, changed))
            .collect()
    }

    fn normalized_type_param_bound(
        &mut self,
        bound: TypeParamBound<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> TypeParamBound<'cx> {
        match bound {
            TypeParamBound::Trait(path) => {
                TypeParamBound::Trait(self.normalized_path(path, active, changed))
            }
            TypeParamBound::Unsupported => TypeParamBound::Unsupported,
        }
    }

    fn normalized_path(
        &mut self,
        path: Path<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> Path<'cx> {
        Path {
            segments: path
                .segments
                .into_iter()
                .map(|segment| {
                    let args = segment
                        .args
                        .into_iter()
                        .map(|arg| self.normalized_generic_argument(arg, active, changed))
                        .collect();
                    PathSegment {
                        name: segment.name,
                        args,
                    }
                })
                .collect(),
        }
    }
}

impl<'cx> Index<TypeId> for InferDb<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

/// Constant facts supplied by top-level orchestration to improve inference precision.
///
/// `syn-sem-infer` does not run constant evaluation itself. Instead, `syn-sem-top` can evaluate
/// constants in a separate phase and feed back only the facts inference needs for a pass.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InferConstFacts {
    expr_values: Map<hir::ExprId, InferConstValue>,
    def_values: Map<DefId, InferConstValue>,
}

impl InferConstFacts {
    /// Records a known const value for a HIR expression.
    pub fn insert_expr_value(&mut self, expr: hir::ExprId, value: InferConstValue) -> bool {
        self.expr_values.insert(expr, value).is_none()
    }

    /// Returns a known const value for a HIR expression.
    pub fn const_expr_value(&self, expr: hir::ExprId) -> Option<InferConstValue> {
        self.expr_values.get(&expr).copied()
    }

    /// Records a known const value for a definition.
    pub fn insert_def_value(&mut self, def: DefId, value: InferConstValue) -> bool {
        self.def_values.insert(def, value).is_none()
    }

    /// Returns a known const value for a definition.
    pub fn const_def_value(&self, def: DefId) -> Option<InferConstValue> {
        self.def_values.get(&def).copied()
    }

    /// Returns a known integer value converted to the expected integer type.
    pub fn expect_integer<T>(&self, expr: hir::ExprId) -> Option<T>
    where
        T: TryFrom<u128>,
    {
        let InferConstValue::Int(value) = self.const_expr_value(expr)? else {
            return None;
        };
        T::try_from(value.value).ok()
    }
}

/// Constant value input used by inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InferConstValue {
    /// Integer const value.
    Int(ConstInt),
    /// Boolean const value.
    Bool(bool),
}

/// Integer constant value plus its current primitive type state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstInt {
    /// Integer value before signed-width interpretation.
    pub value: u128,
    /// Current integer primitive, such as `abstract_int`, `i32`, or `usize`.
    pub primitive: crate::PrimitiveType,
}

#[cfg(test)]
mod tests {
    use crate::*;
    use syn_sem_ast::{self as ast, SourceKind, SyntaxCx};
    use syn_sem_common::CommonCx;
    use syn_sem_hir as hir;
    use syn_sem_name::{AstNodeId, DefId, NameDb, NameDbBuilder, Namespace, ResolveResult};

    fn infer_ty_ids<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (hir::Hir<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_file(file_path, source_text, SourceKind::Virtual)
            .unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameDb::default();
        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, &names, &InferConstFacts::default());
        (hir, infer)
    }

    fn infer_collected_types<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (hir::Hir<'cx>, NameDb<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_file(file_path, source_text, SourceKind::Virtual)
            .unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names =
            NameDbBuilder::build([ast::SourceInput { file_path, file }], [file_path]).unwrap();
        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, &names, &InferConstFacts::default());
        (hir, names, infer)
    }

    fn root_type_def<'cx>(names: &NameDb<'cx>, name: syn_sem_name::Name<'cx>) -> DefId {
        let ResolveResult::Found(def) =
            names.resolve_type_path(names.crate_scope(), [name].into_iter())
        else {
            panic!("expected root type definition");
        };
        def
    }

    #[test]
    fn const_facts_preserve_integer_primitive_state() {
        // Proves const facts preserve both integer value and primitive type state.
        let mut facts = InferConstFacts::default();
        let expr = hir::ExprId::new(0);
        let def = syn_sem_name::DefId::new(0);
        let value = InferConstValue::Int(ConstInt {
            value: 3,
            primitive: PrimitiveType::Usize,
        });

        assert!(facts.insert_expr_value(expr, value));
        assert!(facts.insert_def_value(def, value));
        assert_eq!(facts.const_expr_value(expr), Some(value));
        assert_eq!(facts.const_def_value(def), Some(value));
        assert_eq!(facts.expect_integer::<usize>(expr), Some(3));
    }

    fn struct_field_path_type<'a, 'cx>(
        hir: &'a hir::Hir<'cx>,
        infer: &'a InferDb<'cx>,
    ) -> &'a PathType<'cx> {
        let id = struct_field_type_id(hir, infer);
        let Type::Path(path) = &infer[id] else {
            panic!("struct field type should lower to path type");
        };
        path
    }

    fn struct_field_type_id<'cx>(hir: &hir::Hir<'cx>, infer: &InferDb<'cx>) -> TypeId {
        let hir_type = hir
            .types()
            .iter()
            .find(|source| matches!(source.source, hir::TypeSource::StructField))
            .unwrap();
        infer.type_for_hir_type(hir_type.id).unwrap()
    }

    fn struct_field_hir_types<'cx>(hir: &hir::Hir<'cx>, struct_name: &str) -> Vec<hir::TypeId> {
        let item = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == struct_name))
            .expect("struct item should be represented");
        let hir::ItemKind::Struct { fields, .. } = &item.kind else {
            panic!("item should be represented as a struct");
        };
        fields.iter().map(|field| hir[*field].ty).collect()
    }

    fn struct_field_path_resolution<'cx>(
        hir: &hir::Hir<'cx>,
        infer: &InferDb<'cx>,
    ) -> PathTypeResolution {
        struct_field_path_type(hir, infer).resolution.clone()
    }

    #[test]
    fn lowers_single_segment_primitives_to_primitive_types() {
        // Proves single-segment Rust primitive names lower to primitive inference types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_ty_ids(
            &ccx,
            &scx,
            "fn f(a: bool, b: char, c: str, d: i32, e: usize, f: f64) {}",
        );

        let lowered: Vec<_> = hir
            .types()
            .iter()
            .filter_map(|hir_type| infer.type_for_hir_type(hir_type.id))
            .map(|id| &infer[id])
            .filter_map(|ty| match ty {
                Type::Primitive(primitive) => Some(*primitive),
                _ => None,
            })
            .collect();

        assert!(lowered.contains(&PrimitiveType::Bool));
        assert!(lowered.contains(&PrimitiveType::Char));
        assert!(lowered.contains(&PrimitiveType::Str));
        assert!(lowered.contains(&PrimitiveType::I32));
        assert!(lowered.contains(&PrimitiveType::Usize));
        assert!(lowered.contains(&PrimitiveType::F64));
    }

    #[test]
    fn keeps_non_primitive_paths_as_paths() {
        // Proves qualified primitive-looking names remain unresolved path types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_ty_ids(&ccx, &scx, "struct S { field: crate::usize }");

        let hir_type = hir
            .types()
            .iter()
            .find(|source| matches!(source.source, hir::TypeSource::StructField))
            .unwrap();
        let id = infer.type_for_hir_type(hir_type.id).unwrap();
        let Type::Path(path) = &infer[id] else {
            panic!("qualified primitive-looking path should remain a path");
        };

        assert_eq!(path.path.segments.len(), 2);
        assert_eq!(path.path.segments[0].name.as_ref(), "crate");
        assert_eq!(path.path.segments[1].name.as_ref(), "usize");
        assert_eq!(path.resolution, PathTypeResolution::Unresolved);
    }

    #[test]
    fn keeps_distinct_infer_type_occurrences_separate() {
        // Proves separate `_` occurrences receive distinct inference type ids.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_ty_ids(&ccx, &scx, "struct S { first: _, second: _ }");

        let field_ty_ids = struct_field_hir_types(&hir, "S");
        let [first, second] = field_ty_ids.as_slice() else {
            panic!("struct should have two field types");
        };
        let first = infer.type_for_hir_type(*first).unwrap();
        let second = infer.type_for_hir_type(*second).unwrap();

        assert_ne!(
            first, second,
            "independent `_` type occurrences must not share an inference TypeId"
        );
        assert!(matches!(infer[first], Type::Infer));
        assert!(matches!(infer[second], Type::Infer));
    }

    #[test]
    fn interning_keeps_unresolved_paths_separate() {
        // Proves unresolved paths are not structurally interned across source scopes.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_, mut infer) = infer_ty_ids(&ccx, &scx, "struct S;");
        let unresolved = || {
            Type::Path(PathType {
                qself: None,
                path: Path {
                    segments: vec![PathSegment {
                        name: ccx.intern("Unknown"),
                        args: Vec::new(),
                    }],
                },
                resolution: PathTypeResolution::Unresolved,
            })
        };

        let first = infer.intern_type(unresolved());
        let second = infer.intern_type(unresolved());

        assert_ne!(
            first, second,
            "unresolved paths can depend on source scope and must not share a TypeId"
        );
    }

    #[test]
    fn interning_deeply_shares_container_types() {
        // Proves container types can share when their inner types are deeply shareable.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, mut infer) = infer_ty_ids(&ccx, &scx, "struct S { first: u32, second: u32 }");

        let field_ty_ids = struct_field_hir_types(&hir, "S");
        let [first, second] = field_ty_ids.as_slice() else {
            panic!("struct should have two field types");
        };
        let first = infer.type_for_hir_type(*first).unwrap();
        let second = infer.type_for_hir_type(*second).unwrap();
        assert_ne!(first, second, "HIR occurrences should remain distinct");

        let first_ref = infer.intern_type(Type::Reference {
            elem: first,
            is_mut: false,
        });
        let second_ref = infer.intern_type(Type::Reference {
            elem: second,
            is_mut: false,
        });

        assert_eq!(
            first_ref, second_ref,
            "container types should share when their inner TypeIds are deeply shareable"
        );
    }

    #[test]
    fn classifies_nominal_path_targets() {
        // Proves resolved nominal paths are classified as nominal inference types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let local = ccx.intern("Local");
        let (hir, names, infer) =
            infer_collected_types(&ccx, &scx, "struct Local; struct S { field: Local }");
        let local_def = root_type_def(&names, local);

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Nominal(local_def)
        );
    }

    #[test]
    fn classifies_generic_type_parameters_separately_from_nominal_types() {
        // Proves generic type parameters are classified separately from nominal paths.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let (hir, names, infer) = infer_collected_types(&ccx, &scx, "struct S<T> { field: T }");
        let s_def = root_type_def(&names, ccx.intern("S"));
        let generic_scope = names
            .def_generic_scope(s_def)
            .expect("struct should have a generic scope");
        let t_def = names
            .binding(generic_scope, Namespace::Type, t)
            .and_then(|binding| binding.single())
            .expect("generic type should have a definition");

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::GenericParam(t_def)
        );
    }

    #[test]
    fn classifies_plain_associated_type_targets_without_solver_obligations() {
        // Proves plain associated type paths classify as projections without solver obligations.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let item = ccx.intern("Item");
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            "trait Tr { type Item; } use Tr::Item; struct S { field: Item }",
        );
        let tr_def = root_type_def(&names, ccx.intern("Tr"));
        let ResolveResult::Found(item_def) = names.member(tr_def, Namespace::Type, item) else {
            panic!("trait should have an associated type");
        };

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Projection(ProjectionType {
                assoc: item_def,
                self_: None,
                trait_: None,
            })
        );
        assert!(infer.projections.obligations.is_empty());
    }

    #[test]
    fn lowers_qualified_associated_type_paths_for_projection_solving() {
        // Proves qualified associated type paths produce projection solver matches.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let trait_name = ccx.intern("Trait");
        let item = ccx.intern("Item");
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            "mod a { pub mod b { pub trait Trait { type Item; } } } struct S<T> { field: <T as a::b::Trait>::Item }",
        );
        let s_def = root_type_def(&names, ccx.intern("S"));
        let generic_scope = names
            .def_generic_scope(s_def)
            .expect("struct should have a generic scope");
        let t_def = names
            .binding(generic_scope, Namespace::Type, t)
            .and_then(|binding| binding.single())
            .expect("generic type should have a definition");
        let ResolveResult::Found(trait_def) = names.resolve_type_path(
            names.crate_scope(),
            [ccx.intern("crate"), a, b, trait_name].into_iter(),
        ) else {
            panic!("trait path should resolve");
        };
        let ResolveResult::Found(item_def) = names.member(trait_def, Namespace::Type, item) else {
            panic!("trait should have an associated type");
        };

        let path = struct_field_path_type(&hir, &infer);
        let projection = struct_field_type_id(&hir, &infer);
        let qself = path.qself.expect("qualified path should lower qself");
        let trait_ = qself
            .trait_
            .expect("qualified path should lower trait path");

        assert_eq!(path.path.segments.len(), 4);
        assert_eq!(path.path.segments[0].name.as_ref(), "a");
        assert_eq!(path.path.segments[1].name.as_ref(), "b");
        assert_eq!(path.path.segments[2].name.as_ref(), "Trait");
        assert_eq!(path.path.segments[3].name.as_ref(), "Item");
        assert!(matches!(
            infer[qself.self_],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let Type::Path(lowered_trait_path) = &infer[trait_] else {
            panic!("qself trait type should lower to path type");
        };
        assert_eq!(lowered_trait_path.path.segments.len(), 3);
        assert_eq!(lowered_trait_path.path.segments[0].name.as_ref(), "a");
        assert_eq!(lowered_trait_path.path.segments[1].name.as_ref(), "b");
        assert_eq!(lowered_trait_path.path.segments[2].name.as_ref(), "Trait");
        assert!(matches!(
            infer[trait_],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == trait_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc: item_def,
                self_: Some(qself.self_),
                trait_: Some(trait_),
            })
        );
        assert_eq!(
            infer.projections.projection_matches,
            &[ProjectionMatch {
                projection,
                self_: qself.self_,
                assoc: item_def,
                trait_,
            }]
        );
    }

    #[test]
    fn lowers_traitless_qualified_associated_type_paths_as_projection_matches() {
        // Proves traitless qualified associated type paths use trait bounds for projection matches.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let iterator = ccx.intern("Iterator");
        let item = ccx.intern("Item");
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            "trait PathItem { type Item; } use PathItem::Item; trait Iterator { type Item; } struct S<T: Iterator> { field: <T>::Item }",
        );
        let s_def = root_type_def(&names, ccx.intern("S"));
        let generic_scope = names
            .def_generic_scope(s_def)
            .expect("struct should have a generic scope");
        let t_def = names
            .binding(generic_scope, Namespace::Type, t)
            .and_then(|binding| binding.single())
            .expect("generic type should have a definition");
        let iterator_def = root_type_def(&names, iterator);
        let ResolveResult::Found(iterator_item_def) =
            names.member(iterator_def, Namespace::Type, item)
        else {
            panic!("iterator should have an associated type");
        };
        let path_item_def = root_type_def(&names, ccx.intern("PathItem"));
        let ResolveResult::Found(item_def) = names.member(path_item_def, Namespace::Type, item)
        else {
            panic!("path item trait should have an associated type");
        };

        let path = struct_field_path_type(&hir, &infer);
        let projection = struct_field_type_id(&hir, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");

        assert_eq!(path.path.segments.len(), 1);
        assert_eq!(path.path.segments[0].name.as_ref(), "Item");
        assert_eq!(qself.trait_, None);
        assert!(matches!(
            infer[qself.self_],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc: item_def,
                self_: Some(qself.self_),
                trait_: None,
            })
        );
        assert_eq!(
            infer.projections.obligations,
            &[ProjectionObligation {
                projection,
                assoc: item_def,
                self_: qself.self_,
                trait_: None,
            }]
        );
        let [bound] = infer.trait_bounds.as_slice() else {
            panic!("expected one trait bound");
        };
        assert!(matches!(
            infer[bound.subject],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert!(matches!(
            infer[bound.trait_],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(
            infer.projections.projection_matches,
            &[ProjectionMatch {
                projection,
                self_: qself.self_,
                assoc: iterator_item_def,
                trait_: bound.trait_,
            }]
        );
    }

    #[test]
    fn skips_projection_matches_when_candidate_trait_has_no_associated_type_member() {
        // Proves candidate traits without the requested associated type do not create matches.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let display = ccx.intern("Display");
        let item = ccx.intern("Item");
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            "trait PathItem { type Item; } use PathItem::Item; trait Display {} struct S<T: Display> { field: <T>::Item }",
        );
        let s_def = root_type_def(&names, ccx.intern("S"));
        let generic_scope = names
            .def_generic_scope(s_def)
            .expect("struct should have a generic scope");
        let t_def = names
            .binding(generic_scope, Namespace::Type, t)
            .and_then(|binding| binding.single())
            .expect("generic type should have a definition");
        let display_def = root_type_def(&names, display);
        let path_item_def = root_type_def(&names, ccx.intern("PathItem"));
        assert!(matches!(
            names.member(path_item_def, Namespace::Type, item),
            ResolveResult::Found(_)
        ));

        let path = struct_field_path_type(&hir, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");
        assert!(matches!(
            infer[qself.self_],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let [bound] = infer.trait_bounds.as_slice() else {
            panic!("expected one trait bound");
        };
        assert!(matches!(
            infer[bound.trait_],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == display_def
        ));
        assert_eq!(infer.projections.projection_matches, &[]);
    }

    #[test]
    fn lowers_impl_associated_type_assignments_as_solver_input() {
        // Proves impl associated type assignments become projection solver input facts.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let source_text = "struct Vec; trait Iterator { type Item; } impl Iterator for Vec { type Item = u32; } struct Output { field: <Vec as Iterator>::Item }";
        let source_text = ccx.intern(source_text);
        scx.parse_file(file_path, source_text, SourceKind::Virtual)
            .unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let ast::Item::Struct(_) = &file.items[0] else {
            panic!("expected struct item");
        };
        let ast::Item::Trait(trait_item) = &file.items[1] else {
            panic!("expected trait item");
        };
        let ast::TraitItem::Type(_) = &trait_item.items[0] else {
            panic!("expected trait associated type");
        };
        let ast::Item::Impl(impl_item) = &file.items[2] else {
            panic!("expected impl item");
        };
        let ast::ImplItem::Type(_) = &impl_item.items[0] else {
            panic!("expected impl associated type");
        };

        let names =
            NameDbBuilder::build([ast::SourceInput { file_path, file }], [file_path]).unwrap();
        let vec_def = names
            .def_for_ast_node(AstNodeId::from_ref(&file.items[0]))
            .expect("Vec should have a definition");
        let iterator_def = names
            .def_for_ast_node(AstNodeId::from_ref(&file.items[1]))
            .expect("Iterator should have a definition");
        let trait_assoc_def = names
            .def_for_ast_node(AstNodeId::from_ref(&trait_item.items[0]))
            .expect("trait associated type should have a definition");
        let impl_item_def = names
            .def_for_ast_node(AstNodeId::from_ref(&impl_item.items[0]))
            .expect("impl associated type should have a definition");
        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(&ccx, &hir, &names, &InferConstFacts::default());
        let [impl_assoc_type] = infer.impl_assoc_types.as_slice() else {
            panic!("expected one impl associated type");
        };

        let hir::ItemKind::Impl {
            trait_,
            self_,
            items,
            ..
        } = &hir.items()[2].kind
        else {
            panic!("expected represented impl");
        };
        assert!(trait_.is_some());
        let [assoc_item] = items.as_slice() else {
            panic!("expected one represented impl item");
        };
        assert!(matches!(
            hir[*assoc_item].kind,
            hir::AssocItemKind::ImplType { .. }
        ));
        assert_eq!(hir[*assoc_item].def, Some(impl_item_def));
        assert_eq!(impl_assoc_type.assoc, trait_assoc_def);
        assert_eq!(
            impl_assoc_type.impl_self,
            infer.type_for_hir_type(*self_).unwrap()
        );
        assert!(matches!(
            infer[impl_assoc_type.impl_self],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == vec_def
        ));
        assert!(matches!(
            infer[impl_assoc_type.trait_],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(
            infer[impl_assoc_type.value_ty],
            Type::Primitive(PrimitiveType::U32)
        );

        let projection_path = struct_field_path_type(&hir, &infer);
        let qself = projection_path
            .qself
            .expect("projection field should lower qself");
        let projection = struct_field_type_id(&hir, &infer);
        assert_eq!(
            infer.projections.normalizations,
            &[ProjectionNormalization {
                projection,
                self_: qself.self_,
                assoc: trait_assoc_def,
                trait_: qself.trait_.unwrap(),
                value_ty: impl_assoc_type.value_ty,
            }]
        );
        let normalizations = infer
            .projections
            .normalizations_for(projection)
            .collect::<Vec<_>>();
        assert_eq!(normalizations.len(), 1);
        assert_eq!(
            infer[normalizations[0].value_ty],
            Type::Primitive(PrimitiveType::U32)
        );
    }

    #[test]
    fn normalizes_user_defined_projection_without_generic_impl_self_bindings() {
        // Proves non-generic user-defined trait projections normalize without binding work.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct A;
            struct B;

            trait Combine<Rhs> {
                type Output;
            }

            impl Combine<B> for A {
                type Output = usize;
            }

            struct Holder {
                field: <A as Combine<B>>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Holder");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Holder should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty_id).unwrap();
        let [impl_self_match] = infer.projections.impl_self_matches.as_slice() else {
            panic!("non-generic impl self should match projection self once");
        };
        let [normalization] = infer.projections.normalizations.as_slice() else {
            panic!("non-generic impl self projection should normalize once");
        };

        assert_eq!(impl_self_match.projection_self, normalization.self_);
        assert!(infer.projections.impl_self_generic_bindings.is_empty());
        assert!(infer.projections.type_substitutions.is_empty());
        assert_eq!(
            infer.normalized_projection_type(projection),
            Some(normalization.value_ty)
        );
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(normalization.value_ty)
        );
        assert_eq!(
            infer[normalization.value_ty],
            Type::Primitive(PrimitiveType::Usize)
        );
    }

    #[test]
    fn derives_generic_impl_self_binding_and_substitution() {
        // Proves generic impl-self bindings substitute associated type values.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Output {
                field: <Vec<u32> as Iterator>::Item,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Output");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Output should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty_id).unwrap();
        let [impl_self_match] = infer.projections.impl_self_matches.as_slice() else {
            panic!("generic impl self should match projection self once");
        };
        let [binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("generic impl self should create one type binding");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("generic impl associated type should create one substitution");
        };

        assert_eq!(binding.projection_self, impl_self_match.projection_self);
        assert_eq!(binding.impl_self, impl_self_match.impl_self);
        assert_eq!(substitution.projection_self, binding.projection_self);
        assert_eq!(substitution.impl_self, binding.impl_self);
        assert_eq!(substitution.generic, binding.generic);
        assert_eq!(substitution.arg, binding.arg);
        assert_eq!(substitution.substituted, binding.arg);
        assert_eq!(
            infer.normalized_projection_type(projection),
            Some(substitution.substituted)
        );
        assert_eq!(
            infer[substitution.substituted],
            Type::Primitive(PrimitiveType::U32)
        );
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(substitution.substituted)
        );
    }

    #[test]
    fn derives_impl_self_binding_to_projection_generic() {
        // Proves projection-side generics can bind to impl-self generics.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Identity {
                type Output;
            }

            impl<T> Identity for Vec<T> {
                type Output = T;
            }

            struct Holder<U> {
                field: <Vec<U> as Identity>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Holder");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Holder should have one field type");
        };
        let [binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("projection generic should be bound to impl self generic");
        };
        let normalized = infer
            .normalized_projection_type_for_hir_type(*field_ty_id)
            .expect("projection generic impl self projection should normalize");

        assert_eq!(binding.arg, normalized);
        assert_generic_type_name(&names, &infer, binding.generic, "T");
        assert_generic_type_name(&names, &infer, binding.arg, "U");
        assert_generic_type_name(&names, &infer, normalized, "U");
    }

    #[test]
    fn classifies_projection_normalization_query_results() {
        // Proves projection normalization queries report known, ambiguous, and non-projection states.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                projected: <Vec<u32> as Iterator>::Item,
                plain: u32,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [projected_ty_id, plain_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let projection = infer.type_for_hir_type(*projected_ty_id).unwrap();
        let plain = infer.type_for_hir_type(*plain_ty_id).unwrap();
        let known = infer
            .normalized_projection_type(projection)
            .expect("projection should have one normalization");

        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(known)
        );
        assert_eq!(
            infer.projection_normalization(plain),
            ProjectionNormalizationResult::NotProjection
        );

        let ambiguous_value_ty_id = infer.intern_type(Type::Primitive(PrimitiveType::Bool));
        let ProjectionNormalizationResult::Known(_) = infer.projection_normalization(projection)
        else {
            panic!("projection should start with one known normalization");
        };
        let existing = infer.projections.normalizations[0];
        infer
            .projections
            .normalizations
            .push(ProjectionNormalization {
                value_ty: ambiguous_value_ty_id,
                ..existing
            });
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Ambiguous
        );
        assert_eq!(infer.normalized_projection_type(projection), None);
        assert_eq!(infer.normalized_type(projection), projection);
    }

    #[test]
    fn reports_projection_without_known_normalization() {
        // Proves unresolved projections report the no-normalization state.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            struct Result {
                field: <Vec<u32> as Iterator>::Item,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty_id).unwrap();

        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::NoNormalization
        );
        assert_eq!(infer.normalized_projection_type(projection), None);
        assert_eq!(infer.normalized_type(projection), projection);
    }

    #[test]
    fn derives_selected_substitution_from_multiple_generic_bindings() {
        // Proves associated type values select the matching generic binding from multiple args.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Pair<K, V>;

            trait Select {
                type Output;
            }

            impl<K, V> Select for Pair<K, V> {
                type Output = V;
            }

            struct Result {
                field: <Pair<u32, bool> as Select>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_first_binding, _second_binding] =
            infer.projections.impl_self_generic_bindings.as_slice()
        else {
            panic!("Pair<K, V> should create two type bindings");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("type Output = V should create one selected substitution");
        };

        assert!(infer
            .projections
            .impl_self_generic_bindings
            .iter()
            .any(|binding| {
                substitution.projection_self == binding.projection_self
                    && substitution.impl_self == binding.impl_self
                    && substitution.generic == binding.generic
                    && substitution.arg == binding.arg
            }));
        assert_eq!(
            infer.normalized_projection_type_for_hir_type(*field_ty_id),
            Some(substitution.substituted)
        );
        assert_eq!(
            infer[substitution.substituted],
            Type::Primitive(PrimitiveType::Bool)
        );
    }

    #[test]
    fn reuses_representative_type_for_repeated_concrete_shape_terms() {
        // Proves repeated concrete shape terms reuse a representative type during normalization.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Pair<K, V>;

            trait First {
                type Output;
            }

            impl<K, V> First for Pair<K, V> {
                type Output = K;
            }

            struct Result {
                field: <Pair<u32, u32> as First>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty_id).unwrap();
        let [first_binding, second_binding] =
            infer.projections.impl_self_generic_bindings.as_slice()
        else {
            panic!("Pair<K, V> should create two type bindings");
        };
        assert_eq!(
            infer[first_binding.arg],
            Type::Primitive(PrimitiveType::U32)
        );
        assert_eq!(
            infer[second_binding.arg],
            Type::Primitive(PrimitiveType::U32)
        );

        let normalized = infer
            .normalized_projection_type(projection)
            .expect("projection should normalize through repeated concrete shape terms");
        assert_eq!(infer[normalized], Type::Primitive(PrimitiveType::U32));
    }

    #[test]
    fn keeps_type_shape_term_bindings_local_to_each_projection_self() {
        // Proves type-shape bindings stay local to each projection self type.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Pair<K, V>;

            trait First {
                type Output;
            }

            impl<K, V> First for Pair<K, V> {
                type Output = K;
            }

            struct Result {
                first: <Pair<u32, bool> as First>::Output,
                second: <Pair<bool, u32> as First>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [first_ty_id, second_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let first_projection = infer.type_for_hir_type(*first_ty_id).unwrap();
        let second_projection = infer.type_for_hir_type(*second_ty_id).unwrap();
        let first_normalization = infer
            .projections
            .normalizations_for(first_projection)
            .next()
            .expect("first projection should normalize");
        let second_normalization = infer
            .projections
            .normalizations_for(second_projection)
            .next()
            .expect("second projection should normalize");
        let first_args = path_type_args(&infer, first_normalization.self_, "Pair");
        let second_args = path_type_args(&infer, second_normalization.self_, "Pair");

        assert_eq!(first_normalization.value_ty, first_args[0]);
        assert_eq!(second_normalization.value_ty, second_args[0]);
    }

    #[test]
    fn rejects_repeated_impl_self_generic_mismatch() {
        // Proves repeated impl-self generics must match consistently before normalization.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Pair<K, V>;

            trait SamePair {
                type Output;
            }

            impl<T> SamePair for Pair<T, T> {
                type Output = T;
            }

            struct Result {
                same: <Pair<u32, u32> as SamePair>::Output,
                different: <Pair<u32, bool> as SamePair>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [same_ty_id, different_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let same = infer.type_for_hir_type(*same_ty_id).unwrap();
        let different = infer.type_for_hir_type(*different_ty_id).unwrap();

        let [binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("repeated impl self generic should create one type binding");
        };
        assert_eq!(infer[binding.arg], Type::Primitive(PrimitiveType::U32));
        assert_eq!(
            infer.projection_normalization(same),
            ProjectionNormalizationResult::Known(
                infer
                    .normalized_projection_type(same)
                    .expect("matching repeated generic should normalize")
            )
        );
        assert_eq!(
            infer.projection_normalization(different),
            ProjectionNormalizationResult::NoNormalization
        );
    }

    #[test]
    fn derives_nested_impl_self_binding_with_logic_unification() {
        // Proves nested impl-self shapes bind generics through logic unification.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Option<Vec<T>> {
                type Output = T;
            }

            struct Result {
                field: <Option<Vec<u32>> as Wrap>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("nested impl self should create one type binding");
        };
        let normalized = infer
            .normalized_projection_type_for_hir_type(*field_ty_id)
            .expect("nested impl self projection should normalize");
        assert_primitive_type(&infer, normalized, PrimitiveType::U32);
    }

    #[test]
    fn derives_composite_impl_self_binding_with_logic_unification() {
        // Proves composite impl-self shapes substitute nested path values.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Wrapper<T>;

            trait Identity {
                type Output;
            }

            impl<T> Identity for Wrapper<T> {
                type Output = T;
            }

            struct Result {
                field: <Wrapper<Vec<u32>> as Identity>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("composite impl self should create one type binding");
        };
        let normalized = infer
            .normalized_projection_type_for_hir_type(*field_ty_id)
            .expect("composite impl self projection should normalize");
        assert_path_of(&infer, normalized, "Vec", PrimitiveType::U32);
    }

    #[test]
    fn derives_container_impl_self_bindings_with_logic_unification() {
        // Proves reference, tuple, and slice impl-self shapes bind through logic unification.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            trait Identity {
                type Output;
            }

            impl<T> Identity for &T {
                type Output = T;
            }

            impl<T> Identity for (T, bool) {
                type Output = T;
            }

            impl<T> Identity for [T] {
                type Output = T;
            }

            struct Result {
                reference: <&u32 as Identity>::Output,
                tuple: <(u32, bool) as Identity>::Output,
                slice: <[u32] as Identity>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [reference_ty_id, tuple_ty_id, slice_ty_id] = fields.as_slice() else {
            panic!("Result should have three field types");
        };
        let reference = infer
            .normalized_projection_type_for_hir_type(*reference_ty_id)
            .expect("reference impl self projection should normalize");
        let tuple = infer
            .normalized_projection_type_for_hir_type(*tuple_ty_id)
            .expect("tuple impl self projection should normalize");
        let slice = infer
            .normalized_projection_type_for_hir_type(*slice_ty_id)
            .expect("slice impl self projection should normalize");
        assert_primitive_type(&infer, reference, PrimitiveType::U32);
        assert_primitive_type(&infer, tuple, PrimitiveType::U32);
        assert_primitive_type(&infer, slice, PrimitiveType::U32);
    }

    #[test]
    fn matches_literal_const_args_in_impl_self_logic_unification() {
        // Proves literal const args participate in impl-self matching.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Array<T, const N: usize>;

            trait FixedThree {
                type Output;
            }

            impl<T> FixedThree for Array<T, 3> {
                type Output = T;
            }

            struct Result {
                matching: <Array<u32, 3> as FixedThree>::Output,
                different: <Array<u32, 4> as FixedThree>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [matching_ty_id, different_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let matching = infer.type_for_hir_type(*matching_ty_id).unwrap();
        let different = infer.type_for_hir_type(*different_ty_id).unwrap();
        let normalized = infer
            .normalized_projection_type(matching)
            .expect("matching const literal impl self projection should normalize");

        assert_primitive_type(&infer, normalized, PrimitiveType::U32);
        assert_eq!(
            infer.projection_normalization(different),
            ProjectionNormalizationResult::NoNormalization
        );
    }

    #[test]
    fn derives_assoc_type_arg_impl_self_binding_with_logic_unification() {
        // Proves associated type args inside impl-self shapes bind generic values.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Uses<I>;

            trait Iterator {
                type Item;
            }

            trait Identity {
                type Output;
            }

            impl<T> Identity for Uses<Iterator<Item = T>> {
                type Output = T;
            }

            struct Result {
                matching: <Uses<Iterator<Item = u32>> as Identity>::Output,
                different: <Uses<Iterator<Item = bool>> as Identity>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [matching_ty_id, different_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let matching = infer
            .normalized_projection_type_for_hir_type(*matching_ty_id)
            .expect("associated type arg impl self projection should normalize");
        let different = infer
            .normalized_projection_type_for_hir_type(*different_ty_id)
            .expect("different associated type arg should still bind its own value");

        assert_primitive_type(&infer, matching, PrimitiveType::U32);
        assert_primitive_type(&infer, different, PrimitiveType::Bool);
    }

    #[test]
    fn matches_assoc_const_arg_in_impl_self_logic_unification() {
        // Proves associated const args constrain impl-self matching.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Uses<I, T>;

            trait Flag {
                const PANIC: bool;
            }

            trait Identity {
                type Output;
            }

            impl<T> Identity for Uses<Flag<PANIC = false>, T> {
                type Output = T;
            }

            struct Result {
                matching: <Uses<Flag<PANIC = false>, u32> as Identity>::Output,
                different: <Uses<Flag<PANIC = true>, u32> as Identity>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [matching_ty_id, different_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let matching = infer
            .normalized_projection_type_for_hir_type(*matching_ty_id)
            .expect("associated const arg impl self projection should normalize");
        let different = infer.type_for_hir_type(*different_ty_id).unwrap();

        assert_primitive_type(&infer, matching, PrimitiveType::U32);
        assert_eq!(
            infer.projection_normalization(different),
            ProjectionNormalizationResult::NoNormalization
        );
    }

    #[test]
    fn derives_nested_generic_value_substitution() {
        // Proves generic substitutions recurse into associated type value shapes.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            struct Result {
                field: <Vec<u32> as Wrap>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_binding] = infer.projections.impl_self_generic_bindings.as_slice() else {
            panic!("Vec<T> should create one type binding");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("nested generic associated type should create one substitution");
        };

        assert_eq!(
            substitution.projection_self,
            infer.projections.impl_self_generic_bindings[0].projection_self
        );
        assert_eq!(
            substitution.impl_self,
            infer.projections.impl_self_generic_bindings[0].impl_self
        );
        assert_eq!(
            infer.normalized_projection_type_for_hir_type(*field_ty_id),
            Some(substitution.substituted)
        );
        assert_option_of(&infer, substitution.substituted, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_generic_argument() {
        // Proves recursive normalization rewrites projections inside generic arguments.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                field: Option<<Vec<u32> as Iterator>::Item>,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let shallow = infer
            .shallow_normalized_type_for_hir_type(*field_ty_id)
            .expect("Result.field type should be lowered");
        let normalized = infer
            .normalized_type_for_hir_type(*field_ty_id)
            .expect("Result.field type should be lowered");

        assert_ne!(shallow, normalized);
        assert_option_of(&infer, normalized, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_type_containers() {
        // Proves recursive normalization rewrites projections inside type containers.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                reference: &<Vec<u32> as Iterator>::Item,
                tuple: (<Vec<u32> as Iterator>::Item, bool),
                array: [<Vec<u32> as Iterator>::Item; 1],
                slice_ref: &[<Vec<u32> as Iterator>::Item],
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [reference_ty_id, tuple_ty_id, array_ty_id, slice_ref_ty_id] = fields.as_slice() else {
            panic!("Result should have four field types");
        };

        let reference = infer
            .normalized_type_for_hir_type(*reference_ty_id)
            .unwrap();
        let Type::Reference { elem, is_mut } = infer[reference] else {
            panic!("reference field should normalize to a reference type");
        };
        assert!(!is_mut);
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let tuple = infer.normalized_type_for_hir_type(*tuple_ty_id).unwrap();
        let Type::Tuple { elems } = &infer[tuple] else {
            panic!("tuple field should normalize to a tuple type");
        };
        let [first, second] = elems.as_slice() else {
            panic!("tuple field should have two elements");
        };
        assert_primitive_type(&infer, *first, PrimitiveType::U32);
        assert_primitive_type(&infer, *second, PrimitiveType::Bool);

        let array = infer.normalized_type_for_hir_type(*array_ty_id).unwrap();
        let Type::Array { elem, .. } = infer[array] else {
            panic!("array field should normalize to an array type");
        };
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let slice_ref = infer
            .normalized_type_for_hir_type(*slice_ref_ty_id)
            .unwrap();
        let Type::Reference { elem: slice, .. } = infer[slice_ref] else {
            panic!("slice_ref field should normalize to a reference type");
        };
        let Type::Slice { elem } = infer[slice] else {
            panic!("slice_ref element should normalize to a slice type");
        };
        assert_primitive_type(&infer, elem, PrimitiveType::U32);
    }

    #[test]
    fn caches_recursive_normalization_results() {
        // Proves recursive normalization reuses cached normalized type results.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, mut infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Option<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                field: Option<<Vec<u32> as Iterator>::Item>,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [field_ty_id] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let before = infer.types.len();
        let first = infer.normalized_type_for_hir_type(*field_ty_id).unwrap();
        let after_first = infer.types.len();
        let second = infer.normalized_type_for_hir_type(*field_ty_id).unwrap();
        let after_second = infer.types.len();

        assert_eq!(first, second);
        assert!(after_first >= before);
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
        // Proves generic substitutions stay tied to their originating impl-self match.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, _names, infer) = infer_collected_types(
            &ccx,
            &scx,
            r#"
            struct Vec<T>;
            struct Box<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            impl<U> Wrap for Box<U> {
                type Output = Option<U>;
            }

            struct Result {
                vec_field: <Vec<u32> as Wrap>::Output,
                box_field: <Box<bool> as Wrap>::Output,
            }
            "#,
        );

        let fields = struct_field_hir_types(&hir, "Result");
        let [vec_field_ty_id, box_field_ty_id] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let [_vec_substitution, _box_substitution] =
            infer.projections.type_substitutions.as_slice()
        else {
            panic!("each generic impl associated type should create one substitution");
        };

        for substitution in &infer.projections.type_substitutions {
            assert!(infer
                .projections
                .impl_self_generic_bindings
                .iter()
                .any(|binding| {
                    substitution.projection_self == binding.projection_self
                        && substitution.impl_self == binding.impl_self
                        && substitution.generic == binding.generic
                        && substitution.arg == binding.arg
                }));
        }
        let vec_normalized_ty_id = infer
            .normalized_projection_type_for_hir_type(*vec_field_ty_id)
            .expect("Vec projection should have one context-matched normalization");
        let box_normalized_ty_id = infer
            .normalized_projection_type_for_hir_type(*box_field_ty_id)
            .expect("Box projection should have one context-matched normalization");
        assert_option_of(&infer, vec_normalized_ty_id, PrimitiveType::U32);
        assert_option_of(&infer, box_normalized_ty_id, PrimitiveType::Bool);
    }

    fn assert_option_of(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        assert_path_of(infer, ty, "Option", expected);
    }

    fn assert_path_of(
        infer: &InferDb<'_>,
        ty: TypeId,
        expected_name: &str,
        expected_arg: PrimitiveType,
    ) {
        let Type::Path(path) = &infer[ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("{expected_name}<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), expected_name);
        let [GenericArg::Type(arg)] = segment.args.as_slice() else {
            panic!("{expected_name}<T> should have one type argument");
        };
        assert_eq!(infer[*arg], Type::Primitive(expected_arg));
    }

    fn path_type_args(infer: &InferDb<'_>, ty: TypeId, expected_name: &str) -> Vec<TypeId> {
        let Type::Path(path) = &infer[ty] else {
            panic!("{expected_name}<...> should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("{expected_name}<...> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), expected_name);
        segment
            .args
            .iter()
            .map(|arg| {
                let GenericArg::Type(ty) = arg else {
                    panic!("{expected_name}<...> should have only type arguments");
                };
                *ty
            })
            .collect()
    }

    fn assert_primitive_type(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        assert_eq!(infer[ty], Type::Primitive(expected));
    }

    fn assert_generic_type_name(
        names: &NameDb<'_>,
        infer: &InferDb<'_>,
        ty: TypeId,
        expected: &str,
    ) {
        let Type::Path(path) = &infer[ty] else {
            panic!("type should be a generic path");
        };
        let PathTypeResolution::GenericParam(def) = path.resolution else {
            panic!("type path should resolve to a generic parameter");
        };
        assert_eq!(names[def].name.unwrap().as_ref(), expected);
    }

    #[test]
    fn preserves_array_length_expression_id() {
        // Proves array length lowering preserves the source HIR expression id.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_ty_ids(&ccx, &scx, "struct S { field: [u8; 3] }");
        let hir_ty_id = struct_field_hir_types(&hir, "S")[0];
        let infer_ty_id = infer.type_for_hir_type(hir_ty_id).unwrap();

        let Type::Array { len, .. } = &infer[infer_ty_id] else {
            panic!("array field should lower to an array type");
        };
        let ArrayLen::Expr(expr) = len else {
            panic!("source array type length should remain a HIR expression");
        };
        let hir::TypeKind::Array {
            len: hir::ArrayLen::Expr(hir_expr),
            ..
        } = hir[hir_ty_id].kind
        else {
            panic!("HIR field should be an array type");
        };
        assert_eq!(*expr, hir_expr);
    }

    #[test]
    fn preserves_const_generic_arguments() {
        // Proves type lowering preserves const generic literal arguments.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_ty_ids(
            &ccx,
            &scx,
            "struct Array<T, const N: usize> { field: T } struct S { field: Array<u8, 3> }",
        );
        let hir_ty_id = struct_field_hir_types(&hir, "S")[0];
        let infer_ty_id = infer.type_for_hir_type(hir_ty_id).unwrap();

        let Type::Path(path) = &infer[infer_ty_id] else {
            panic!("field should lower to a path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Array<u8, 3> should have one path segment");
        };
        let [GenericArg::Type(_), GenericArg::Const(ConstArg::Lit(crate::Lit::Int(value)))] =
            segment.args.as_slice()
        else {
            panic!("Array<u8, 3> should preserve its type and const arguments");
        };
        assert_eq!(value.as_ref(), "3");
    }

    #[test]
    fn preserves_associated_const_arguments() {
        // Proves type lowering preserves associated const arguments on trait bounds.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_hir, infer) = infer_ty_ids(
            &ccx,
            &scx,
            r#"
            trait Trait {
                const PANIC: bool;
            }

            struct S<T: Trait<PANIC = false>> {
                field: T,
            }
            "#,
        );
        let trait_bound = infer.trait_bounds.first().unwrap();
        let Type::Path(trait_) = &infer[trait_bound.trait_] else {
            panic!("trait bound should lower to a path type");
        };
        let [GenericArg::AssocConst { name, value }] = trait_.path.segments[0].args.as_slice()
        else {
            panic!("trait bound should preserve associated const argument");
        };
        assert_eq!(name.as_ref(), "PANIC");
        assert_eq!(*value, ConstArg::Lit(crate::Lit::Bool(false)));
    }

    #[test]
    fn preserves_associated_type_constraint_bounds() {
        // Proves type lowering preserves associated type constraint bounds.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_hir, infer) = infer_ty_ids(
            &ccx,
            &scx,
            r#"
            struct S<T: Iterator<Item: std::fmt::Display>> {
                field: T,
            }
            "#,
        );
        let trait_bound = infer.trait_bounds.first().unwrap();
        let Type::Path(trait_) = &infer[trait_bound.trait_] else {
            panic!("trait bound should lower to a path type");
        };
        let [GenericArg::Constraint { name, bounds }] = trait_.path.segments[0].args.as_slice()
        else {
            panic!("trait bound should preserve associated type constraint");
        };
        assert_eq!(name.as_ref(), "Item");
        let [TypeParamBound::Trait(bound)] = bounds.as_slice() else {
            panic!("constraint should preserve its trait bound");
        };
        assert_eq!(bound.segments[0].name.as_ref(), "std");
        assert_eq!(bound.segments[1].name.as_ref(), "fmt");
        assert_eq!(bound.segments[2].name.as_ref(), "Display");
    }

    #[test]
    fn preserves_ambiguous_and_unresolved_path_states() {
        // Proves path lowering preserves ambiguous and unresolved resolution states.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let maybe = ccx.intern("Maybe");
        let (hir, names, infer) = infer_collected_types(
            &ccx,
            &scx,
            "struct Maybe; enum Maybe {} struct S { field: Maybe }",
        );
        let defs = names
            .binding(names.crate_scope(), Namespace::Type, maybe)
            .expect("duplicate definitions should have a binding")
            .iter()
            .collect::<Vec<_>>();

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Ambiguous(defs)
        );

        let (hir, infer) = infer_ty_ids(&ccx, &scx, "struct S { field: Missing }");

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Unresolved
        );
    }
}
