mod assoc_type_impl;
pub(crate) mod body_type;
mod infer_types;
pub(crate) mod projection;
mod trait_bound;

use crate::{
    logic, AssocTypeImplFact, GenericArgument, Path, PathSegment, PathType, PathTypeResolution,
    ProjectionDb, ProjectionNormalizationResult, ProjectionType, QSelf, TraitBound, TraitBoundFact,
    Type, TypeBounds, TypeId, TypeParamBound,
};
use assoc_type_impl::AssocTypeImplFactCollector;
use body_type::{BodyTypeCollector, BodyTypeDb};
use infer_types::InferTypes;
use projection::ProjectionCollector;
use std::ops::Index;
use syn_sem_common::{CommonCx, Map};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb};
use trait_bound::TraitBoundFactCollector;

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    pub(crate) types: InferTypes<'cx>,
    pub(crate) projections: ProjectionDb,
    pub(crate) body_types: BodyTypeDb,
    pub(crate) trait_bound_facts: Vec<TraitBoundFact>,
    pub(crate) assoc_type_impl_facts: Vec<AssocTypeImplFact>,
    pub(crate) recursive_normalizations: Map<TypeId, TypeId>,
}

impl<'cx> InferDb<'cx> {
    /// Builds inference type facts from HIR and name-resolution data.
    pub fn analyze(ccx: &'cx CommonCx, hir: &hir::Hir<'cx>, names: &NameDb<'cx>) -> Self {
        let mut db = InferDbBuilder::new(hir, names).build();
        logic::derive(ccx, &mut db, names);
        db
    }

    /// Returns the inference type linked to a HIR type occurrence.
    pub fn type_for_hir_type(&self, hir_type: hir::TypeId) -> Option<TypeId> {
        self.types.type_for_hir_type(hir_type)
    }

    /// Returns the resolved concrete type linked to a HIR expression occurrence.
    pub fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.body_types.type_for_hir_expr(hir_expr)
    }

    /// Returns the resolved concrete type linked to a definition, when body inference found one.
    pub fn type_for_def(&self, def: DefId) -> Option<TypeId> {
        self.body_types.type_for_def(def)
    }

    /// Returns the shallow normalized inference type linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered.
    #[cfg(test)]
    pub(crate) fn shallow_normalized_type_for_hir_type(
        &self,
        hir_type: hir::TypeId,
    ) -> Option<TypeId> {
        self.type_for_hir_type(hir_type)
            .map(|ty| self.shallow_normalized_type(ty))
    }

    /// Returns the unique normalized projection value linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered, is not a
    /// projection with a known normalization, or currently has multiple possible normalizations.
    #[cfg(test)]
    pub(crate) fn normalized_projection_type_for_hir_type(
        &self,
        hir_type: hir::TypeId,
    ) -> Option<TypeId> {
        let ty = self.type_for_hir_type(hir_type)?;
        self.normalized_projection_type(ty)
    }

    /// Returns the recursively normalized inference type linked to a HIR type occurrence.
    ///
    /// This returns `None` when the HIR type occurrence was not lowered.
    pub fn normalized_type_for_hir_type(&mut self, hir_type: hir::TypeId) -> Option<TypeId> {
        let ty = self.type_for_hir_type(hir_type)?;
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
        let Type::Path(path) = &self[ty] else {
            return None;
        };
        Some(&path.resolution)
    }

    /// Returns the nominal definition named by a type, when the type is a nominal path.
    pub fn nominal_def(&self, ty: TypeId) -> Option<DefId> {
        let PathTypeResolution::Nominal(def) = self.path_resolution(ty)? else {
            return None;
        };
        Some(*def)
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

    fn normalized_type_inner(&mut self, ty: TypeId, active: &mut Vec<TypeId>) -> TypeId {
        if active.contains(&ty) {
            return ty;
        }
        active.push(ty);

        match self.projection_normalization(ty) {
            ProjectionNormalizationResult::Known(value_ty) if value_ty != ty => {
                let normalized = self.normalized_type_inner(value_ty, active);
                active.pop();
                return normalized;
            }
            ProjectionNormalizationResult::Known(_)
            | ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => {}
        }

        let normalized = match self[ty].clone() {
            Type::Array { elem, len } => {
                let elem = self.normalized_type_inner(elem, active);
                self.intern_changed_type(ty, Type::Array { elem, len })
            }
            Type::Infer | Type::Primitive(_) => ty,
            Type::Path(path) => {
                let (path, changed) = self.normalized_path_type(path, active);
                if changed {
                    self.intern_type(Type::Path(path))
                } else {
                    ty
                }
            }
            Type::Reference { elem, is_mut } => {
                let elem = self.normalized_type_inner(elem, active);
                self.intern_changed_type(ty, Type::Reference { elem, is_mut })
            }
            Type::Slice { elem } => {
                let elem = self.normalized_type_inner(elem, active);
                self.intern_changed_type(ty, Type::Slice { elem })
            }
            Type::Tuple { elems } => {
                let elems = elems
                    .into_iter()
                    .map(|elem| self.normalized_type_inner(elem, active))
                    .collect();
                self.intern_changed_type(ty, Type::Tuple { elems })
            }
        };

        active.pop();
        normalized
    }

    fn intern_changed_type(&mut self, original: TypeId, ty: Type<'cx>) -> TypeId {
        if self[original] == ty {
            return original;
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
            let self_ty = self.normalized_type_inner(qself.self_ty, active);
            changed |= self_ty != qself.self_ty;
            let trait_ty = qself.trait_ty.map(|trait_ty| {
                let normalized = self.normalized_type_inner(trait_ty, active);
                changed |= normalized != trait_ty;
                normalized
            });
            QSelf { self_ty, trait_ty }
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
        arg: GenericArgument<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> GenericArgument<'cx> {
        match arg {
            GenericArgument::Type(ty) => {
                let normalized = self.normalized_type_inner(ty, active);
                *changed |= normalized != ty;
                GenericArgument::Type(normalized)
            }
            GenericArgument::AssocType { name, ty } => {
                let normalized = self.normalized_type_inner(ty, active);
                *changed |= normalized != ty;
                GenericArgument::AssocType {
                    name,
                    ty: normalized,
                }
            }
            GenericArgument::Const(arg) => GenericArgument::Const(arg),
            GenericArgument::AssocConst { name, value } => {
                GenericArgument::AssocConst { name, value }
            }
            GenericArgument::Constraint { name, bounds } => {
                let bounds = self.normalized_type_bounds(bounds, active, changed);
                GenericArgument::Constraint { name, bounds }
            }
            GenericArgument::Unsupported => GenericArgument::Unsupported,
        }
    }

    fn normalized_type_bounds(
        &mut self,
        bounds: TypeBounds<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> TypeBounds<'cx> {
        TypeBounds {
            bounds: bounds
                .bounds
                .into_iter()
                .map(|bound| self.normalized_type_param_bound(bound, active, changed))
                .collect(),
        }
    }

    fn normalized_type_param_bound(
        &mut self,
        bound: TypeParamBound<'cx>,
        active: &mut Vec<TypeId>,
        changed: &mut bool,
    ) -> TypeParamBound<'cx> {
        match bound {
            TypeParamBound::Trait(bound) => TypeParamBound::Trait(TraitBound {
                path: self.normalized_path(bound.path, active, changed),
            }),
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
        let trait_bound_facts = TraitBoundFactCollector::collect(self.hir, self.names, &mut types);
        types.collect_hir_types(self.hir, self.names);
        let assoc_type_impl_facts =
            AssocTypeImplFactCollector::collect(self.hir, self.names, &mut types);
        let projections = ProjectionCollector::collect(types.as_slice());
        let body_types = BodyTypeCollector::collect(self.hir, self.names, &mut types);

        InferDb {
            types,
            projections,
            body_types,
            trait_bound_facts,
            assoc_type_impl_facts,
            recursive_normalizations: Map::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::*;
    use syn_sem_ast::{self as ast, SyntaxCx};
    use syn_sem_common::CommonCx;
    use syn_sem_hir as hir;
    use syn_sem_name::{
        collect::NameCollector, AstNodeId, DefKind, NameDb, Origin, ScopeKind, Visibility,
    };

    fn infer_types<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (hir::Hir<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameDb::default();
        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, &names);
        (hir, infer)
    }

    fn infer_types_with_names<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
        names: &NameDb<'cx>,
    ) -> (hir::Hir<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let hir = hir::HirBuilder::new(names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, names);
        (hir, infer)
    }

    fn infer_collected_types<'cx>(
        ccx: &'cx CommonCx,
        scx: &'cx SyntaxCx<'cx>,
        source_text: &str,
    ) -> (hir::Hir<'cx>, NameDb<'cx>, InferDb<'cx>) {
        let file_path = ccx.intern("test.rs");
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();
        let names = NameCollector::new([ast::SourceInput { file_path, file }])
            .collect(file_path)
            .unwrap();
        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(ccx, &hir, &names);
        (hir, names, infer)
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
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_types(
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
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_types(&ccx, &scx, "struct S { field: crate::usize }");

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
    fn classifies_nominal_path_targets() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let local = ccx.intern("Local");
        let mut names = NameDb::default();
        let local_def = names.add_def(
            names.root_scope(),
            DefKind::Struct,
            Some(local),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Local }", &names);

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Nominal(local_def)
        );
    }

    #[test]
    fn classifies_generic_type_parameters_separately_from_nominal_types() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let mut names = NameDb::default();
        let t_def = names.add_def(
            names.root_scope(),
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: T }", &names);

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::GenericParam(t_def)
        );
    }

    #[test]
    fn classifies_associated_type_targets_as_projection_candidates() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let item_def = names.add_def(
            names.root_scope(),
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Item }", &names);

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: None,
                trait_ty: None,
            })
        );
    }

    #[test]
    fn lowers_qualified_associated_type_paths_for_projection_solving() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let a = ccx.intern("a");
        let b = ccx.intern("b");
        let trait_name = ccx.intern("Trait");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let a_def = names.add_def(
            root,
            DefKind::Module,
            Some(a),
            Visibility::Public,
            Origin::Untracked,
        );
        let a_scope = names.add_scope(syn_sem_name::ScopeKind::Module, Some(root));
        names.set_path_scope(a_def, a_scope);
        let b_def = names.add_def(
            a_scope,
            DefKind::Module,
            Some(b),
            Visibility::Public,
            Origin::Untracked,
        );
        let b_scope = names.add_scope(syn_sem_name::ScopeKind::Module, Some(a_scope));
        names.set_path_scope(b_def, b_scope);
        let trait_def = names.add_def(
            b_scope,
            DefKind::Trait,
            Some(trait_name),
            Visibility::Public,
            Origin::Untracked,
        );
        let trait_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(b_scope));
        names.set_path_scope(trait_def, trait_scope);
        let item_def = names.add_def(
            trait_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Public,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S { field: <T as a::b::Trait>::Item }",
            &names,
        );

        let path = struct_field_path_type(&hir, &infer);
        let projection = struct_field_type_id(&hir, &infer);
        let qself = path.qself.expect("qualified path should lower qself");
        let trait_ty = qself
            .trait_ty
            .expect("qualified path should lower trait path");

        assert_eq!(path.path.segments.len(), 4);
        assert_eq!(path.path.segments[0].name.as_ref(), "a");
        assert_eq!(path.path.segments[1].name.as_ref(), "b");
        assert_eq!(path.path.segments[2].name.as_ref(), "Trait");
        assert_eq!(path.path.segments[3].name.as_ref(), "Item");
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let Type::Path(lowered_trait_path) = &infer[trait_ty] else {
            panic!("qself trait type should lower to path type");
        };
        assert_eq!(lowered_trait_path.path.segments.len(), 3);
        assert_eq!(lowered_trait_path.path.segments[0].name.as_ref(), "a");
        assert_eq!(lowered_trait_path.path.segments[1].name.as_ref(), "b");
        assert_eq!(lowered_trait_path.path.segments[2].name.as_ref(), "Trait");
        assert!(matches!(
            infer[trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == trait_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: Some(trait_ty),
            })
        );
        assert_eq!(
            infer.projections.candidates,
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty,
            }]
        );
        assert_eq!(
            infer.projections.matches,
            &[ProjectionMatch {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty,
            }]
        );
    }

    #[test]
    fn lowers_traitless_qualified_associated_type_paths_as_projection_candidates() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let iterator = ccx.intern("Iterator");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_def = names.add_def(
            root,
            DefKind::Trait,
            Some(iterator),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(root));
        names.set_path_scope(iterator_def, iterator_scope);
        let iterator_item_def = names.add_def(
            iterator_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S<T: Iterator> { field: <T>::Item }",
            &names,
        );

        let path = struct_field_path_type(&hir, &infer);
        let projection = struct_field_type_id(&hir, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");

        assert_eq!(path.path.segments.len(), 1);
        assert_eq!(path.path.segments[0].name.as_ref(), "Item");
        assert_eq!(qself.trait_ty, None);
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert_eq!(
            path.resolution,
            PathTypeResolution::Projection(ProjectionType {
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: None,
            })
        );
        assert_eq!(
            infer.projections.obligations,
            &[ProjectionObligation {
                projection,
                assoc_type: item_def,
                self_ty: Some(qself.self_ty),
                trait_ty: None,
            }]
        );
        let [bound] = infer.trait_bound_facts.as_slice() else {
            panic!("expected one trait bound fact");
        };
        assert!(matches!(
            infer[bound.subject],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        assert!(matches!(
            infer[bound.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(
            infer.projections.candidates,
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty: bound.trait_ty,
            }]
        );
        assert_eq!(
            infer.projections.matches,
            &[ProjectionMatch {
                projection,
                self_ty: qself.self_ty,
                assoc_type: iterator_item_def,
                trait_ty: bound.trait_ty,
            }]
        );
    }

    #[test]
    fn skips_projection_matches_when_candidate_trait_has_no_associated_type_member() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let t = ccx.intern("T");
        let display = ccx.intern("Display");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let t_def = names.add_def(
            root,
            DefKind::GenericType,
            Some(t),
            Visibility::Private,
            Origin::Untracked,
        );
        let display_def = names.add_def(
            root,
            DefKind::Trait,
            Some(display),
            Visibility::Private,
            Origin::Untracked,
        );
        let display_scope = names.add_scope(syn_sem_name::ScopeKind::Trait, Some(root));
        names.set_path_scope(display_def, display_scope);
        let item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(
            &ccx,
            &scx,
            "struct S<T: Display> { field: <T>::Item }",
            &names,
        );

        let projection = struct_field_type_id(&hir, &infer);
        let path = struct_field_path_type(&hir, &infer);
        let qself = path
            .qself
            .expect("traitless qualified path should lower qself");
        assert!(matches!(
            infer[qself.self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::GenericParam(def),
                ..
            }) if def == t_def
        ));
        let [bound] = infer.trait_bound_facts.as_slice() else {
            panic!("expected one trait bound fact");
        };
        assert!(matches!(
            infer[bound.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == display_def
        ));
        assert_eq!(
            infer.projections.candidates,
            &[ProjectionCandidate {
                projection,
                self_ty: qself.self_ty,
                assoc_type: item_def,
                trait_ty: bound.trait_ty,
            }]
        );
        assert_eq!(infer.projections.matches, &[]);
    }

    #[test]
    fn lowers_impl_associated_type_assignments_as_solver_input() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let file_path = ccx.intern("test.rs");
        let source_text = "struct Vec; trait Iterator { type Item; } impl Iterator for Vec { type Item = u32; } struct Output { field: <Vec as Iterator>::Item }";
        let source_text = ccx.intern(source_text);
        scx.parse_virtual_file(file_path, source_text).unwrap();
        let file = scx.lookup_source(file_path).unwrap().ast();

        let iterator = ccx.intern("Iterator");
        let vec = ccx.intern("Vec");
        let item = ccx.intern("Item");
        let mut names = NameDb::default();
        let root = names.root_scope();
        let vec_def = names.add_def(
            root,
            DefKind::Struct,
            Some(vec),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_def = names.add_def(
            root,
            DefKind::Trait,
            Some(iterator),
            Visibility::Private,
            Origin::Untracked,
        );
        let iterator_scope = names.add_scope(ScopeKind::Trait, Some(root));
        names.set_path_scope(iterator_def, iterator_scope);
        let trait_assoc_def = names.add_def(
            iterator_scope,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let impl_item_def = names.add_def(
            root,
            DefKind::AssocType,
            Some(item),
            Visibility::Private,
            Origin::Untracked,
        );
        let ast::Item::Struct(_) = &file.items[0] else {
            panic!("expected struct item");
        };
        names.set_def_ast_node(vec_def, AstNodeId::from_ref(&file.items[0]));
        let ast::Item::Trait(trait_item) = &file.items[1] else {
            panic!("expected trait item");
        };
        names.set_def_ast_node(iterator_def, AstNodeId::from_ref(&file.items[1]));
        let ast::TraitItem::Type(_) = &trait_item.items[0] else {
            panic!("expected trait associated type");
        };
        let ast::Item::Impl(impl_item) = &file.items[2] else {
            panic!("expected impl item");
        };
        let ast::ImplItem::Type(_) = &impl_item.items[0] else {
            panic!("expected impl associated type");
        };
        names.set_def_ast_node(impl_item_def, AstNodeId::from_ref(&impl_item.items[0]));

        let hir = hir::HirBuilder::new(&names).build(file_path, file);
        let infer = InferDb::analyze(&ccx, &hir, &names);
        let [fact] = infer.assoc_type_impl_facts.as_slice() else {
            panic!("expected one impl associated type fact");
        };

        let hir::ItemKind::Impl {
            trait_,
            self_ty,
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
        assert_eq!(fact.assoc_type, trait_assoc_def);
        assert_eq!(
            fact.impl_self_ty,
            infer.type_for_hir_type(*self_ty).unwrap()
        );
        assert!(matches!(
            infer[fact.impl_self_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == vec_def
        ));
        assert!(matches!(
            infer[fact.trait_ty],
            Type::Path(PathType {
                resolution: PathTypeResolution::Nominal(def),
                ..
            }) if def == iterator_def
        ));
        assert_eq!(infer[fact.value_ty], Type::Primitive(PrimitiveType::U32));

        let projection_path = struct_field_path_type(&hir, &infer);
        let qself = projection_path
            .qself
            .expect("projection field should lower qself");
        let projection = struct_field_type_id(&hir, &infer);
        assert_eq!(
            infer.projections.normalizations,
            &[ProjectionNormalization {
                projection,
                self_ty: qself.self_ty,
                assoc_type: trait_assoc_def,
                trait_ty: qself.trait_ty.unwrap(),
                value_ty: fact.value_ty,
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
    fn derives_generic_impl_self_binding_and_substitution() {
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
        let [field_ty] = fields.as_slice() else {
            panic!("Output should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty).unwrap();
        let [impl_self_match] = infer.projections.impl_self_matches.as_slice() else {
            panic!("generic impl self should match projection self once");
        };
        let [binding] = infer.projections.type_bindings.as_slice() else {
            panic!("generic impl self should create one type binding");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("generic impl associated type should create one substitution");
        };

        assert_eq!(
            binding.projection_self_ty,
            impl_self_match.projection_self_ty
        );
        assert_eq!(binding.impl_self_ty, impl_self_match.impl_self_ty);
        assert_eq!(substitution.projection_self_ty, binding.projection_self_ty);
        assert_eq!(substitution.impl_self_ty, binding.impl_self_ty);
        assert_eq!(substitution.generic_ty, binding.generic_ty);
        assert_eq!(substitution.arg_ty, binding.arg_ty);
        assert_eq!(substitution.substituted_ty, binding.arg_ty);
        assert_eq!(
            infer.normalized_projection_type(projection),
            Some(substitution.substituted_ty)
        );
        assert_eq!(
            infer[substitution.substituted_ty],
            Type::Primitive(PrimitiveType::U32)
        );
        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::Known(substitution.substituted_ty)
        );
    }

    #[test]
    fn classifies_projection_normalization_query_results() {
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
        let [projected_ty, plain_ty] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let projection = infer.type_for_hir_type(*projected_ty).unwrap();
        let plain = infer.type_for_hir_type(*plain_ty).unwrap();
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

        let ambiguous_value = infer.intern_type(Type::Primitive(PrimitiveType::Bool));
        let ProjectionNormalizationResult::Known(_) = infer.projection_normalization(projection)
        else {
            panic!("projection should start with one known normalization");
        };
        let existing = infer.projections.normalizations[0];
        infer
            .projections
            .normalizations
            .push(ProjectionNormalization {
                value_ty: ambiguous_value,
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
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let projection = infer.type_for_hir_type(*field_ty).unwrap();

        assert_eq!(
            infer.projection_normalization(projection),
            ProjectionNormalizationResult::NoNormalization
        );
        assert_eq!(infer.normalized_projection_type(projection), None);
        assert_eq!(infer.normalized_type(projection), projection);
    }

    #[test]
    fn derives_selected_substitution_from_multiple_generic_bindings() {
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
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_first_binding, _second_binding] = infer.projections.type_bindings.as_slice() else {
            panic!("Pair<K, V> should create two type bindings");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("type Output = V should create one selected substitution");
        };

        assert!(infer.projections.type_bindings.iter().any(|binding| {
            substitution.projection_self_ty == binding.projection_self_ty
                && substitution.impl_self_ty == binding.impl_self_ty
                && substitution.generic_ty == binding.generic_ty
                && substitution.arg_ty == binding.arg_ty
        }));
        assert_eq!(
            infer.normalized_projection_type_for_hir_type(*field_ty),
            Some(substitution.substituted_ty)
        );
        assert_eq!(
            infer[substitution.substituted_ty],
            Type::Primitive(PrimitiveType::Bool)
        );
    }

    #[test]
    fn derives_nested_generic_value_substitution() {
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
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let [_binding] = infer.projections.type_bindings.as_slice() else {
            panic!("Vec<T> should create one type binding");
        };
        let [substitution] = infer.projections.type_substitutions.as_slice() else {
            panic!("nested generic associated type should create one substitution");
        };

        assert_eq!(
            substitution.projection_self_ty,
            infer.projections.type_bindings[0].projection_self_ty
        );
        assert_eq!(
            substitution.impl_self_ty,
            infer.projections.type_bindings[0].impl_self_ty
        );
        assert_eq!(
            infer.normalized_projection_type_for_hir_type(*field_ty),
            Some(substitution.substituted_ty)
        );
        assert_option_of(&infer, substitution.substituted_ty, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_generic_argument() {
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
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let shallow = infer
            .shallow_normalized_type_for_hir_type(*field_ty)
            .expect("Result.field type should be lowered");
        let normalized = infer
            .normalized_type_for_hir_type(*field_ty)
            .expect("Result.field type should be lowered");

        assert_ne!(shallow, normalized);
        assert_option_of(&infer, normalized, PrimitiveType::U32);
    }

    #[test]
    fn recursively_normalizes_projection_inside_type_containers() {
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
        let [reference_ty, tuple_ty, array_ty, slice_ref_ty] = fields.as_slice() else {
            panic!("Result should have four field types");
        };

        let reference = infer.normalized_type_for_hir_type(*reference_ty).unwrap();
        let Type::Reference { elem, is_mut } = infer[reference] else {
            panic!("reference field should normalize to a reference type");
        };
        assert!(!is_mut);
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let tuple = infer.normalized_type_for_hir_type(*tuple_ty).unwrap();
        let Type::Tuple { elems } = &infer[tuple] else {
            panic!("tuple field should normalize to a tuple type");
        };
        let [first, second] = elems.as_slice() else {
            panic!("tuple field should have two elements");
        };
        assert_primitive_type(&infer, *first, PrimitiveType::U32);
        assert_primitive_type(&infer, *second, PrimitiveType::Bool);

        let array = infer.normalized_type_for_hir_type(*array_ty).unwrap();
        let Type::Array { elem, .. } = infer[array] else {
            panic!("array field should normalize to an array type");
        };
        assert_primitive_type(&infer, elem, PrimitiveType::U32);

        let slice_ref = infer.normalized_type_for_hir_type(*slice_ref_ty).unwrap();
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
        let [field_ty] = fields.as_slice() else {
            panic!("Result should have one field type");
        };
        let before = infer.types.len();
        let first = infer.normalized_type_for_hir_type(*field_ty).unwrap();
        let after_first = infer.types.len();
        let second = infer.normalized_type_for_hir_type(*field_ty).unwrap();
        let after_second = infer.types.len();

        assert_eq!(first, second);
        assert!(after_first >= before);
        assert_eq!(after_first, after_second);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
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
        let [vec_field_ty, box_field_ty] = fields.as_slice() else {
            panic!("Result should have two field types");
        };
        let [_vec_substitution, _box_substitution] =
            infer.projections.type_substitutions.as_slice()
        else {
            panic!("each generic impl associated type should create one substitution");
        };

        for substitution in &infer.projections.type_substitutions {
            assert!(infer.projections.type_bindings.iter().any(|binding| {
                substitution.projection_self_ty == binding.projection_self_ty
                    && substitution.impl_self_ty == binding.impl_self_ty
                    && substitution.generic_ty == binding.generic_ty
                    && substitution.arg_ty == binding.arg_ty
            }));
        }
        let vec_normalized_ty = infer
            .normalized_projection_type_for_hir_type(*vec_field_ty)
            .expect("Vec projection should have one context-matched normalization");
        let box_normalized_ty = infer
            .normalized_projection_type_for_hir_type(*box_field_ty)
            .expect("Box projection should have one context-matched normalization");
        assert_option_of(&infer, vec_normalized_ty, PrimitiveType::U32);
        assert_option_of(&infer, box_normalized_ty, PrimitiveType::Bool);
    }

    fn assert_option_of(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        let Type::Path(path) = &infer[ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArgument::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<T> should have one type argument");
        };
        assert_eq!(infer[*arg], Type::Primitive(expected));
    }

    fn assert_primitive_type(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        assert_eq!(infer[ty], Type::Primitive(expected));
    }

    #[test]
    fn preserves_array_length_expression_id() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_types(&ccx, &scx, "struct S { field: [u8; 3] }");
        let hir_type = struct_field_hir_types(&hir, "S")[0];
        let infer_type = infer.type_for_hir_type(hir_type).unwrap();

        let Type::Array { len, .. } = &infer[infer_type] else {
            panic!("array field should lower to an array type");
        };
        let ArrayLen::Expr(expr) = len;
        let hir::TypeKind::Array {
            len: hir::ArrayLen::Expr(hir_expr),
            ..
        } = hir[hir_type].kind
        else {
            panic!("HIR field should be an array type");
        };
        assert_eq!(*expr, hir_expr);
    }

    #[test]
    fn preserves_const_generic_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (hir, infer) = infer_types(
            &ccx,
            &scx,
            "struct Array<T, const N: usize> { field: T } struct S { field: Array<u8, 3> }",
        );
        let hir_type = struct_field_hir_types(&hir, "S")[0];
        let infer_type = infer.type_for_hir_type(hir_type).unwrap();

        let Type::Path(path) = &infer[infer_type] else {
            panic!("field should lower to a path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Array<u8, 3> should have one path segment");
        };
        let [GenericArgument::Type(_), GenericArgument::Const(ConstArg::Lit(crate::Lit::Int(value)))] =
            segment.args.as_slice()
        else {
            panic!("Array<u8, 3> should preserve its type and const arguments");
        };
        assert_eq!(value.as_ref(), "3");
    }

    #[test]
    fn preserves_associated_const_arguments() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_hir, infer) = infer_types(
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
        let fact = infer.trait_bound_facts.first().unwrap();
        let Type::Path(trait_ty) = &infer[fact.trait_ty] else {
            panic!("trait bound should lower to a path type");
        };
        let [GenericArgument::AssocConst { name, value }] =
            trait_ty.path.segments[0].args.as_slice()
        else {
            panic!("trait bound should preserve associated const argument");
        };
        assert_eq!(name.as_ref(), "PANIC");
        assert_eq!(*value, ConstArg::Lit(crate::Lit::Bool(false)));
    }

    #[test]
    fn preserves_associated_type_constraint_bounds() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_hir, infer) = infer_types(
            &ccx,
            &scx,
            r#"
            struct S<T: Iterator<Item: std::fmt::Display>> {
                field: T,
            }
            "#,
        );
        let fact = infer.trait_bound_facts.first().unwrap();
        let Type::Path(trait_ty) = &infer[fact.trait_ty] else {
            panic!("trait bound should lower to a path type");
        };
        let [GenericArgument::Constraint { name, bounds }] =
            trait_ty.path.segments[0].args.as_slice()
        else {
            panic!("trait bound should preserve associated type constraint");
        };
        assert_eq!(name.as_ref(), "Item");
        let [TypeParamBound::Trait(bound)] = bounds.bounds.as_slice() else {
            panic!("constraint should preserve its trait bound");
        };
        assert_eq!(bound.path.segments[0].name.as_ref(), "std");
        assert_eq!(bound.path.segments[1].name.as_ref(), "fmt");
        assert_eq!(bound.path.segments[2].name.as_ref(), "Display");
    }

    #[test]
    fn preserves_ambiguous_and_unresolved_path_states() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let maybe = ccx.intern("Maybe");
        let mut names = NameDb::default();
        let first = names.add_def(
            names.root_scope(),
            DefKind::Struct,
            Some(maybe),
            Visibility::Private,
            Origin::Untracked,
        );
        let second = names.add_def(
            names.root_scope(),
            DefKind::Enum,
            Some(maybe),
            Visibility::Private,
            Origin::Untracked,
        );
        let (hir, infer) = infer_types_with_names(&ccx, &scx, "struct S { field: Maybe }", &names);

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Ambiguous(vec![first, second])
        );

        let (hir, infer) = infer_types(&ccx, &scx, "struct S { field: Missing }");

        assert_eq!(
            struct_field_path_resolution(&hir, &infer),
            PathTypeResolution::Unresolved
        );
    }
}
