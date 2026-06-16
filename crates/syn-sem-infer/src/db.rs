mod projection;

pub use projection::{ProjectionDb, ProjectionNormalizationResult};

use crate::{
    AssocTypeImplFact, BodyBlockFact, BodyLocalFact, GenericArgument, ImplSelfMatch, Path,
    PathSegment, PathType, PathTypeResolution, ProjectionType, QSelf, ResolvedTypeFact, TraitBound,
    TraitBoundFact, Type, TypeBindingFact, TypeBounds, TypeEqualFact, TypeId, TypeParamBound,
    TypeSubstitution,
};
#[cfg(test)]
use crate::{ProjectionCandidate, ProjectionMatch, ProjectionNormalization, ProjectionObligation};
use std::ops::Index;
use syn_sem_common::{CommonCx, Map};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb};

/// Type information collected for upper semantic inference.
#[derive(Debug, Default)]
pub struct InferDb<'cx> {
    pub(crate) types: Vec<Type<'cx>>,
    pub(crate) hir_types: Map<hir::TypeId, TypeId>,
    pub(crate) projections: ProjectionDb,
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
        self.projections.obligations()
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
        self.projections.candidates()
    }

    /// Returns projections matched against concrete associated type members.
    #[cfg(test)]
    pub(crate) fn projection_matches(&self) -> &[ProjectionMatch] {
        self.projections.matches()
    }

    /// Returns projections normalized to impl-provided value types.
    #[cfg(test)]
    pub(crate) fn projection_normalizations(&self) -> &[ProjectionNormalization] {
        self.projections.normalizations()
    }

    /// Returns normalization results for one projection type occurrence.
    #[cfg(test)]
    pub(crate) fn normalizations_for_projection(
        &self,
        projection: TypeId,
    ) -> impl Iterator<Item = &ProjectionNormalization> {
        self.projections.normalizations_for(projection)
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
