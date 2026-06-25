//! Projection-specific facts and query helpers for inference.

use crate::{InferTypes, PathTypeResolution, ProjectionType, Type, TypeId};
use syn_sem_name::DefId;

pub(crate) struct ProjectionCollector;

impl ProjectionCollector {
    pub(crate) fn collect(types: &InferTypes<'_>) -> ProjectionDb {
        let obligations = types
            .iter()
            .filter_map(|(ty_id, ty)| {
                let Type::Path(path_ty) = ty else {
                    return None;
                };
                let PathTypeResolution::Projection(projection) = &path_ty.resolution else {
                    return None;
                };
                let self_ = projection.self_?;

                Some(ProjectionObligation {
                    projection: ty_id,
                    assoc: projection.assoc,
                    self_,
                    trait_: projection.trait_,
                })
            })
            .collect();

        ProjectionDb {
            obligations,
            matches: Vec::new(),
            impl_self_matches: Vec::new(),
            type_bindings: Vec::new(),
            type_substitutions: Vec::new(),
            normalizations: Vec::new(),
        }
    }
}

/// Associated type projection facts owned by inference.
///
/// A projection is the [`TypeId`] for the whole associated-type expression, such as
/// `<T as Iterator>::Item`, `<Vec<u32> as Iterator>::Item`, or `T::Assoc`.
///
/// That type id does not name a nominal type directly. Instead, it means "the `Item` or `Assoc`
/// type selected for this `Self` type, optionally through this trait." The solver records the
/// whole projection, the `Self` type, the associated type definition, and any explicit trait type,
/// then tries to normalize the projection to the value type from a matching impl.
#[derive(Debug, Default)]
pub(crate) struct ProjectionDb {
    /// Projection type occurrences collected during lowering that still need solver work.
    /// * Made in the build stage.
    pub(crate) obligations: Vec<ProjectionObligation>,
    /// Candidate projections matched against concrete associated type members on a trait.
    /// * Made in the derive stage.
    pub(crate) matches: Vec<ProjectionMatch>,
    /// Impl self type matches used for projection normalization.
    /// * Made in the derive stage.
    pub(crate) impl_self_matches: Vec<ImplSelfMatch>,
    /// Type argument bindings discovered from impl self type matches.
    /// * Made in the derive stage.
    pub(crate) type_bindings: Vec<ImplSelfTypeArgBinding>,
    /// Type substitutions used for projection normalization.
    /// * Made in the derive stage.
    pub(crate) type_substitutions: Vec<TypeSubstitution>,
    /// Matched projections rewritten to the value type assigned by an applicable impl.
    /// * Made in the derive stage.
    pub(crate) normalizations: Vec<ProjectionNormalization>,
}

impl ProjectionDb {
    /// Returns associated type projection metadata for a path resolution.
    pub fn projection<'a>(
        &self,
        resolution: Option<&'a PathTypeResolution>,
    ) -> Option<&'a ProjectionType> {
        let PathTypeResolution::Projection(projection) = resolution? else {
            return None;
        };
        Some(projection)
    }

    /// Returns normalization results for one projection type occurrence.
    pub(crate) fn normalizations_for(
        &self,
        projection: TypeId,
    ) -> impl Iterator<Item = &ProjectionNormalization> {
        self.normalizations
            .iter()
            .filter(move |normalization| normalization.projection == projection)
    }

    /// Returns the unique normalized value type for one associated type projection.
    ///
    /// Returns `None` when the projection has no known normalization or when multiple
    /// normalizations are currently possible.
    #[cfg(test)]
    pub(crate) fn normalized_type(
        &self,
        projection: TypeId,
        is_projection: bool,
    ) -> Option<TypeId> {
        match self.normalization(projection, is_projection) {
            ProjectionNormalizationResult::Known(value_ty) => Some(value_ty),
            ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => None,
        }
    }

    /// Returns the normalization query result for one associated type projection.
    pub fn normalization(
        &self,
        projection: TypeId,
        is_projection: bool,
    ) -> ProjectionNormalizationResult {
        if !is_projection {
            return ProjectionNormalizationResult::NotProjection;
        }

        let mut normalizations = self.normalizations_for(projection);
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
}

/// Associated type projection that needs solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionObligation {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Definition that provides the associated type name requested by the projection.
    ///
    /// For `<T>::Item`, this is the requested `Item`, not necessarily the concrete target `Item`
    /// such as `Iterator::Item`. The concrete trait member definition is selected later in
    /// [`ProjectionMatch`].
    pub(crate) assoc: DefId,
    /// Self type for the projection.
    pub(crate) self_: TypeId,
    /// Trait type for the projection, when represented.
    pub(crate) trait_: Option<TypeId>,
}

/// Associated type projection matched against a concrete trait member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionMatch {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Self type for the projection.
    pub(crate) self_: TypeId,
    /// Associated type member found in the candidate trait.
    pub(crate) assoc: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_: TypeId,
}

/// Impl self type pattern matched against a projection self type.
///
/// For `<Vec<u32> as Trait>::Output` and `impl<T> Trait for Vec<T>`, this records that projection
/// self `Vec<u32>` matches impl self `Vec<T>`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplSelfMatch {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self: TypeId,
}

/// Type argument bound to an impl-self generic while matching an impl self type.
///
/// For `<Vec<u32> as Trait>::Output` and `impl<T> Trait for Vec<T>`, this records that generic `T`
/// from impl self `Vec<T>` is bound to projection argument `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplSelfTypeArgBinding {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self: TypeId,
    /// Generic type occurrence from the impl self type, such as `T`.
    pub(crate) generic: TypeId,
    /// Type argument matched for the generic, such as `u32`.
    pub(crate) arg: TypeId,
}

/// Type substitution fact used while normalizing associated type projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeSubstitution {
    /// Self type from the projection that requested the substitution.
    pub(crate) projection_self: TypeId,
    /// Self type from the impl header whose value type is substituted.
    pub(crate) impl_self: TypeId,
    /// Type before substitution, such as `T`.
    pub(crate) value_ty: TypeId,
    /// Generic type occurrence being substituted, such as `T`.
    pub(crate) generic: TypeId,
    /// Type argument used for the generic, such as `u32`.
    pub(crate) arg: TypeId,
    /// Type after substitution, such as `u32`.
    pub(crate) substituted: TypeId,
}

/// Associated type projection normalized to an impl-provided value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionNormalization {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection: TypeId,
    /// Self type for the projection.
    pub(crate) self_: TypeId,
    /// Associated type member used for normalization.
    pub(crate) assoc: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_: TypeId,
    /// Type assigned by the matching impl item.
    pub(crate) value_ty: TypeId,
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
