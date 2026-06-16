use crate::{
    AssocTypeImplFact, BodyBlockFact, BodyLocalFact, ImplSelfMatch, ProjectionCandidate,
    ProjectionMatch, ProjectionNormalization, ProjectionObligation, ResolvedTypeFact,
    TraitBoundFact, TypeBindingFact, TypeEqualFact, TypeSubstitution,
};
use std::ops::Index;
use syn_sem_common::{CommonCx, Map};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb};

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    pub(crate) types: Vec<Type<'cx>>,
    pub(crate) hir_types: Map<hir::TypeId, TypeId>,
    pub(crate) projection_obligations: Vec<ProjectionObligation>,
    pub(crate) trait_bound_facts: Vec<TraitBoundFact>,
    pub(crate) assoc_type_impl_facts: Vec<AssocTypeImplFact>,
    pub(crate) body_block_facts: Vec<BodyBlockFact>,
    pub(crate) body_local_facts: Vec<BodyLocalFact>,
    pub(crate) type_equal_facts: Vec<TypeEqualFact>,
    pub(crate) resolved_type_facts: Vec<ResolvedTypeFact>,
    pub(crate) hir_expr_types: Map<hir::ExprId, TypeId>,
    pub(crate) def_types: Map<DefId, TypeId>,
    pub(crate) impl_self_matches: Vec<ImplSelfMatch>,
    pub(crate) type_binding_facts: Vec<TypeBindingFact>,
    pub(crate) type_substitutions: Vec<TypeSubstitution>,
    pub(crate) projection_candidates: Vec<ProjectionCandidate>,
    pub(crate) projection_matches: Vec<ProjectionMatch>,
    pub(crate) projection_normalizations: Vec<ProjectionNormalization>,
    pub(crate) recursive_normalizations: Map<TypeId, TypeId>,
}

impl<'cx> InferDb<'cx> {
    /// Builds inference type facts from HIR and name-resolution data.
    pub fn analyze(ccx: &'cx CommonCx, hir: &hir::Hir<'cx>, names: &NameDb<'cx>) -> Self {
        crate::inference::analyze(ccx, hir, names)
    }

    /// Returns all collected inference types.
    #[cfg(test)]
    pub(crate) fn types(&self) -> &[Type<'cx>] {
        &self.types
    }

    /// Returns the inference type linked to a HIR type occurrence.
    pub fn type_for_hir_type(&self, hir_type: hir::TypeId) -> Option<TypeId> {
        self.hir_types.get(&hir_type).copied()
    }

    /// Returns the resolved concrete type linked to a HIR expression occurrence.
    pub fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.hir_expr_types.get(&hir_expr).copied()
    }

    /// Returns the resolved concrete type linked to a definition, when body inference found one.
    pub fn type_for_def(&self, def: DefId) -> Option<TypeId> {
        self.def_types.get(&def).copied()
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

    /// Returns associated type projections that still need solver work.
    #[cfg(test)]
    pub(crate) fn projection_obligations(&self) -> &[ProjectionObligation] {
        &self.projection_obligations
    }

    /// Returns trait bounds collected as solver input facts.
    #[cfg(test)]
    pub(crate) fn trait_bound_facts(&self) -> &[TraitBoundFact] {
        &self.trait_bound_facts
    }

    /// Returns associated type assignments collected from trait impls.
    #[cfg(test)]
    pub(crate) fn assoc_type_impl_facts(&self) -> &[AssocTypeImplFact] {
        &self.assoc_type_impl_facts
    }

    /// Returns lowered block facts collected from HIR body lowering.
    #[cfg(test)]
    pub(crate) fn body_block_facts(&self) -> &[BodyBlockFact] {
        &self.body_block_facts
    }

    /// Returns lowered local facts collected from HIR body lowering.
    #[cfg(test)]
    pub(crate) fn body_local_facts(&self) -> &[BodyLocalFact] {
        &self.body_local_facts
    }

    /// Returns body-local type equality facts.
    #[cfg(test)]
    pub(crate) fn type_equal_facts(&self) -> &[TypeEqualFact] {
        &self.type_equal_facts
    }

    /// Returns concrete body-local type resolutions derived from equality facts.
    #[cfg(test)]
    pub(crate) fn resolved_type_facts(&self) -> &[ResolvedTypeFact] {
        &self.resolved_type_facts
    }

    /// Returns impl self type matches used for projection normalization.
    #[cfg(test)]
    pub(crate) fn impl_self_matches(&self) -> &[ImplSelfMatch] {
        &self.impl_self_matches
    }

    /// Returns generic type bindings discovered from impl self type matches.
    #[cfg(test)]
    pub(crate) fn type_binding_facts(&self) -> &[TypeBindingFact] {
        &self.type_binding_facts
    }

    /// Returns type substitutions used for projection normalization.
    #[cfg(test)]
    pub(crate) fn type_substitutions(&self) -> &[TypeSubstitution] {
        &self.type_substitutions
    }

    /// Returns projection candidates derived from obligations and known trait bounds.
    #[cfg(test)]
    pub(crate) fn projection_candidates(&self) -> &[ProjectionCandidate] {
        &self.projection_candidates
    }

    /// Returns projections matched against concrete associated type members.
    #[cfg(test)]
    pub(crate) fn projection_matches(&self) -> &[ProjectionMatch] {
        &self.projection_matches
    }

    /// Returns projections normalized to impl-provided value types.
    #[cfg(test)]
    pub(crate) fn projection_normalizations(&self) -> &[ProjectionNormalization] {
        &self.projection_normalizations
    }

    /// Returns normalization results for one projection type occurrence.
    pub(crate) fn normalizations_for_projection(
        &self,
        projection: TypeId,
    ) -> impl Iterator<Item = &ProjectionNormalization> {
        self.projection_normalizations
            .iter()
            .filter(move |normalization| normalization.projection == projection)
    }

    /// Returns the unique normalized value type for one associated type projection.
    ///
    /// Returns `None` when the projection has no known normalization or when multiple
    /// normalizations are currently possible.
    #[cfg(test)]
    pub(crate) fn normalized_projection_type(&self, projection: TypeId) -> Option<TypeId> {
        match self.projection_normalization(projection) {
            ProjectionNormalizationResult::Known(value_ty) => Some(value_ty),
            ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => None,
        }
    }

    /// Returns the normalization query result for one associated type projection.
    pub fn projection_normalization(&self, projection: TypeId) -> ProjectionNormalizationResult {
        if self.projection(projection).is_none() {
            return ProjectionNormalizationResult::NotProjection;
        }

        let mut normalizations = self.normalizations_for_projection(projection);
        let Some(value_ty) = normalizations
            .next()
            .map(|normalization| normalization.value_ty)
        else {
            return ProjectionNormalizationResult::NoNormalization;
        };
        if normalizations.next().is_some() {
            return ProjectionNormalizationResult::Ambiguous;
        }
        ProjectionNormalizationResult::Known(value_ty)
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
        if let Some(index) = self.types.iter().position(|existing| existing == &ty) {
            return TypeId::new(index);
        }
        let id = TypeId::new(self.types.len());
        self.types.push(ty);
        id
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
        let PathTypeResolution::Projection(projection) = self.path_resolution(ty)? else {
            return None;
        };
        Some(projection)
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

/// Result of asking whether one projection type can normalize to a value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionNormalizationResult {
    /// The projection has one known normalized value type.
    Known(TypeId),
    /// The queried type is not an associated type projection.
    NotProjection,
    /// The queried type is a projection, but no normalization is known.
    NoNormalization,
    /// The queried projection currently has multiple possible normalizations.
    Ambiguous,
}

impl<'cx> Index<TypeId> for InferDb<'cx> {
    type Output = Type<'cx>;

    fn index(&self, id: TypeId) -> &Self::Output {
        &self.types[id.index()]
    }
}

syn_sem_macros::define_id! {
    {
        /// Stable identity for one inference type.
        pub(crate) TypeId
    }
}

/// Type shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type<'cx> {
    /// Fixed-length array type.
    Array {
        /// Element type.
        elem: TypeId,
        /// Array length expression shape.
        len: ArrayLen,
    },
    /// Inferred type placeholder.
    Infer,
    /// Primitive Rust type.
    Primitive(PrimitiveType),
    /// Path type.
    Path(PathType<'cx>),
    /// Borrowed reference type.
    Reference {
        /// Referenced type.
        elem: TypeId,
        /// Whether the reference is mutable.
        is_mut: bool,
    },
    /// Dynamically sized slice type.
    Slice {
        /// Element type.
        elem: TypeId,
    },
    /// Tuple type.
    Tuple {
        /// Tuple element types.
        elems: Vec<TypeId>,
    },
}

/// Primitive Rust type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    /// `bool`.
    Bool,
    /// `char`.
    Char,
    /// `str`.
    Str,
    /// `i8`.
    I8,
    /// `i16`.
    I16,
    /// `i32`.
    I32,
    /// `i64`.
    I64,
    /// `i128`.
    I128,
    /// `isize`.
    Isize,
    /// `u8`.
    U8,
    /// `u16`.
    U16,
    /// `u32`.
    U32,
    /// `u64`.
    U64,
    /// `u128`.
    U128,
    /// `usize`.
    Usize,
    /// `f32`.
    F32,
    /// `f64`.
    F64,
}

impl PrimitiveType {
    pub(crate) fn from_hir_path(path: &[hir::PathSegment<'_>]) -> Option<Self> {
        let [segment] = path else {
            return None;
        };
        if !segment.args.is_empty() {
            return None;
        }
        Self::from_str(segment.name.as_ref())
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "bool" => Some(Self::Bool),
            "char" => Some(Self::Char),
            "str" => Some(Self::Str),
            "i8" => Some(Self::I8),
            "i16" => Some(Self::I16),
            "i32" => Some(Self::I32),
            "i64" => Some(Self::I64),
            "i128" => Some(Self::I128),
            "isize" => Some(Self::Isize),
            "u8" => Some(Self::U8),
            "u16" => Some(Self::U16),
            "u32" => Some(Self::U32),
            "u64" => Some(Self::U64),
            "u128" => Some(Self::U128),
            "usize" => Some(Self::Usize),
            "f32" => Some(Self::F32),
            "f64" => Some(Self::F64),
            _ => None,
        }
    }
}

/// Path type used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathType<'cx> {
    /// Qualified self type, when the source used qualified path syntax.
    pub qself: Option<QSelf>,
    /// Path naming the type.
    pub path: Path<'cx>,
    /// Current resolution state for this type path.
    pub resolution: PathTypeResolution,
}

/// Qualified self type for an inference path type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QSelf {
    /// Self type written inside `<...>`.
    pub self_ty: TypeId,
    /// Trait path in `<Self as Trait>`, when present.
    pub trait_ty: Option<TypeId>,
}

/// Resolution state for a non-primitive path type.
///
/// This records the best current classification without pretending that name lookup is full Rust
/// type resolution. Solver-backed generic substitution, qualified paths, and projection
/// normalization can refine this later.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathTypeResolution {
    /// Path names a nominal type definition.
    Nominal(DefId),
    /// Path names a generic type parameter.
    GenericParam(DefId),
    /// Path denotes an associated type projection.
    Projection(ProjectionType),
    /// Multiple candidates matched the path.
    Ambiguous(Vec<DefId>),
    /// No target is known for the path yet.
    Unresolved,
}

/// Associated type projection known to path type resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionType {
    /// Associated type definition selected for the projection.
    pub assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub self_ty: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub trait_ty: Option<TypeId>,
}

/// Type path used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path<'cx> {
    /// Path segments in source order.
    pub segments: Vec<PathSegment<'cx>>,
}

/// One type path segment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathSegment<'cx> {
    /// Segment name.
    pub name: syn_sem_name::Name<'cx>,
    /// Generic arguments on this segment.
    pub args: Vec<GenericArgument<'cx>>,
}

/// Generic argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericArgument<'cx> {
    /// Type argument.
    Type(TypeId),
    /// Const expression argument.
    Const(ConstArg<'cx>),
    /// Associated type equality.
    AssocType {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned type.
        ty: TypeId,
    },
    /// Associated const equality.
    AssocConst {
        /// Associated const name.
        name: syn_sem_name::Name<'cx>,
        /// Assigned const value.
        value: ConstArg<'cx>,
    },
    /// Associated type constraint.
    Constraint {
        /// Associated type name.
        name: syn_sem_name::Name<'cx>,
        /// Source bounds.
        bounds: TypeBounds<'cx>,
    },
    /// Unsupported argument form.
    Unsupported,
}

/// Array length expression shape used by inference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayLen {
    /// Length is still a HIR expression.
    Expr(hir::ExprId),
}

impl ArrayLen {
    pub(crate) fn from_hir(len: hir::ArrayLen) -> Self {
        match len {
            hir::ArrayLen::Expr(expr) => Self::Expr(expr),
        }
    }
}

/// Const argument shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstArg<'cx> {
    /// Literal const argument.
    Lit(Lit<'cx>),
    /// Path const argument.
    Path(Path<'cx>),
    /// Const expression argument.
    Expr(hir::ExprId),
}

/// Literal shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Lit<'cx> {
    /// Integer literal stored as normalized base-10 digits.
    Int(syn_sem_common::InternedStr<'cx>),
    /// Floating-point literal stored as normalized base-10 digits.
    Float(syn_sem_common::InternedStr<'cx>),
    /// Boolean literal.
    Bool(bool),
}

impl<'cx> Lit<'cx> {
    pub(crate) fn from_hir(lit: &hir::Lit<'cx>) -> Self {
        match lit {
            hir::Lit::Int(value) => Self::Int(*value),
            hir::Lit::Float(value) => Self::Float(*value),
            hir::Lit::Bool(value) => Self::Bool(*value),
        }
    }
}

/// Type bounds attached to an associated type constraint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeBounds<'cx> {
    /// Bounds in source order.
    pub bounds: Vec<TypeParamBound<'cx>>,
}

/// Type parameter bound shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeParamBound<'cx> {
    /// Trait bound.
    Trait(TraitBound<'cx>),
    /// Unsupported bound form.
    Unsupported,
}

/// Trait bound shape used by inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraitBound<'cx> {
    /// Trait path.
    pub path: Path<'cx>,
}
