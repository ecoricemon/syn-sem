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

                Some(ProjectionObligation {
                    projection_ty_id: ty_id,
                    assoc_type: projection.assoc_type,
                    self_ty_id: projection.self_ty_id,
                    trait_ty_id: projection.trait_ty_id,
                })
            })
            .collect();

        ProjectionDb {
            obligations,
            candidates: Vec::new(),
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
    pub(crate) obligations: Vec<ProjectionObligation>,
    /// Candidate trait selections that may provide the requested associated type.
    pub(crate) candidates: Vec<ProjectionCandidate>,
    /// Candidate projections matched against concrete associated type members on a trait.
    pub(crate) matches: Vec<ProjectionMatch>,
    /// Impl self type matches used for projection normalization.
    pub(crate) impl_self_matches: Vec<ImplSelfMatch>,
    /// Generic type bindings discovered from impl self type matches.
    pub(crate) type_bindings: Vec<TypeBindingFact>,
    /// Type substitutions used for projection normalization.
    pub(crate) type_substitutions: Vec<TypeSubstitution>,
    /// Matched projections rewritten to the value type assigned by an applicable impl.
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
        projection_ty_id: TypeId,
    ) -> impl Iterator<Item = &ProjectionNormalization> {
        self.normalizations
            .iter()
            .filter(move |normalization| normalization.projection_ty_id == projection_ty_id)
    }

    /// Returns the unique normalized value type for one associated type projection.
    ///
    /// Returns `None` when the projection has no known normalization or when multiple
    /// normalizations are currently possible.
    #[cfg(test)]
    pub(crate) fn normalized_type(
        &self,
        projection_ty_id: TypeId,
        is_projection: bool,
    ) -> Option<TypeId> {
        match self.normalization(projection_ty_id, is_projection) {
            ProjectionNormalizationResult::Known(value_ty_id) => Some(value_ty_id),
            ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => None,
        }
    }

    /// Returns the normalization query result for one associated type projection.
    pub fn normalization(
        &self,
        projection_ty_id: TypeId,
        is_projection: bool,
    ) -> ProjectionNormalizationResult {
        if !is_projection {
            return ProjectionNormalizationResult::NotProjection;
        }

        let mut normalizations = self.normalizations_for(projection_ty_id);
        let Some(value_ty_id) = normalizations
            .next()
            .map(|normalization| normalization.value_ty_id)
        else {
            return ProjectionNormalizationResult::NoNormalization;
        };
        if normalizations.next().is_some() {
            return ProjectionNormalizationResult::Ambiguous;
        }
        ProjectionNormalizationResult::Known(value_ty_id)
    }
}

/// Associated type projection that needs solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionObligation {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_ty_id: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub(crate) self_ty_id: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub(crate) trait_ty_id: Option<TypeId>,
}

/// Candidate trait selected for an associated type projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionCandidate {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_ty_id: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty_id: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Candidate trait type that may provide the associated type.
    pub(crate) trait_ty_id: TypeId,
}

/// Associated type projection matched against a concrete trait member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionMatch {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_ty_id: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty_id: TypeId,
    /// Associated type member found in the candidate trait.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_ty_id: TypeId,
}

/// Impl self type pattern matched against a projection self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplSelfMatch {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_ty_id: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_ty_id: TypeId,
}

/// Generic type binding discovered while matching an impl self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeBindingFact {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_ty_id: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_ty_id: TypeId,
    /// Generic type occurrence from the impl self type, such as `T`.
    pub(crate) generic_ty_id: TypeId,
    /// Type argument matched for the generic, such as `u32`.
    pub(crate) arg_ty_id: TypeId,
}

/// Type substitution fact used while normalizing associated type projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeSubstitution {
    /// Self type from the projection that requested the substitution.
    pub(crate) projection_self_ty_id: TypeId,
    /// Self type from the impl header whose value type is substituted.
    pub(crate) impl_self_ty_id: TypeId,
    /// Type before substitution, such as `T`.
    pub(crate) value_ty_id: TypeId,
    /// Generic type occurrence being substituted, such as `T`.
    pub(crate) generic_ty_id: TypeId,
    /// Type argument used for the generic, such as `u32`.
    pub(crate) arg_ty_id: TypeId,
    /// Type after substitution, such as `u32`.
    pub(crate) substituted_ty_id: TypeId,
}

/// Associated type projection normalized to an impl-provided value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionNormalization {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_ty_id: TypeId,
    /// Self type for the projection.
    pub(crate) self_ty_id: TypeId,
    /// Associated type member used for normalization.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_ty_id: TypeId,
    /// Type assigned by the matching impl item.
    pub(crate) value_ty_id: TypeId,
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
