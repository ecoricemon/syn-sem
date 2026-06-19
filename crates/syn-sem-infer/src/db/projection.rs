//! Projection-specific facts and query helpers for inference.

use crate::{InferTypes, PathType, PathTypeResolution, ProjectionType, Type, TypeId};
use syn_sem_name::DefId;

pub(crate) struct ProjectionCollector;

impl ProjectionCollector {
    pub(crate) fn collect(types: &InferTypes<'_>) -> ProjectionDb {
        let mut projections = ProjectionDb::default();

        for (tid, ty) in types.iter() {
            let Type::Path(PathType {
                resolution: PathTypeResolution::Projection(projection),
                ..
            }) = ty
            else {
                continue;
            };

            projections.obligations.push(ProjectionObligation {
                projection_tid: tid,
                assoc_type: projection.assoc_type,
                self_tid: projection.self_tid,
                trait_tid: projection.trait_tid,
            });
        }

        projections
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
        projection_tid: TypeId,
    ) -> impl Iterator<Item = &ProjectionNormalization> {
        self.normalizations
            .iter()
            .filter(move |normalization| normalization.projection_tid == projection_tid)
    }

    /// Returns the unique normalized value type for one associated type projection.
    ///
    /// Returns `None` when the projection has no known normalization or when multiple
    /// normalizations are currently possible.
    #[cfg(test)]
    pub(crate) fn normalized_type(
        &self,
        projection_tid: TypeId,
        is_projection: bool,
    ) -> Option<TypeId> {
        match self.normalization(projection_tid, is_projection) {
            ProjectionNormalizationResult::Known(value_tid) => Some(value_tid),
            ProjectionNormalizationResult::NotProjection
            | ProjectionNormalizationResult::NoNormalization
            | ProjectionNormalizationResult::Ambiguous => None,
        }
    }

    /// Returns the normalization query result for one associated type projection.
    pub fn normalization(
        &self,
        projection_tid: TypeId,
        is_projection: bool,
    ) -> ProjectionNormalizationResult {
        if !is_projection {
            return ProjectionNormalizationResult::NotProjection;
        }

        let mut normalizations = self.normalizations_for(projection_tid);
        let Some(value_tid) = normalizations
            .next()
            .map(|normalization| normalization.value_tid)
        else {
            return ProjectionNormalizationResult::NoNormalization;
        };
        if normalizations.next().is_some() {
            return ProjectionNormalizationResult::Ambiguous;
        }
        ProjectionNormalizationResult::Known(value_tid)
    }
}

/// Associated type projection that needs solver work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionObligation {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_tid: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Self type for the projection, when represented.
    pub(crate) self_tid: Option<TypeId>,
    /// Trait type for the projection, when represented.
    pub(crate) trait_tid: Option<TypeId>,
}

/// Candidate trait selected for an associated type projection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionCandidate {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_tid: TypeId,
    /// Self type for the projection.
    pub(crate) self_tid: TypeId,
    /// Associated type definition selected by name lookup.
    pub(crate) assoc_type: DefId,
    /// Candidate trait type that may provide the associated type.
    pub(crate) trait_tid: TypeId,
}

/// Associated type projection matched against a concrete trait member.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionMatch {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_tid: TypeId,
    /// Self type for the projection.
    pub(crate) self_tid: TypeId,
    /// Associated type member found in the candidate trait.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_tid: TypeId,
}

/// Impl self type pattern matched against a projection self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplSelfMatch {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_tid: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_tid: TypeId,
}

/// Generic type binding discovered while matching an impl self type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeBindingFact {
    /// Self type from the projection, such as `Vec<u32>`.
    pub(crate) projection_self_tid: TypeId,
    /// Self type from the impl header, such as `Vec<T>`.
    pub(crate) impl_self_tid: TypeId,
    /// Generic type occurrence from the impl self type, such as `T`.
    pub(crate) generic_tid: TypeId,
    /// Type argument matched for the generic, such as `u32`.
    pub(crate) arg_tid: TypeId,
}

/// Type substitution fact used while normalizing associated type projections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeSubstitution {
    /// Self type from the projection that requested the substitution.
    pub(crate) projection_self_tid: TypeId,
    /// Self type from the impl header whose value type is substituted.
    pub(crate) impl_self_tid: TypeId,
    /// Type before substitution, such as `T`.
    pub(crate) value_tid: TypeId,
    /// Generic type occurrence being substituted, such as `T`.
    pub(crate) generic_tid: TypeId,
    /// Type argument used for the generic, such as `u32`.
    pub(crate) arg_tid: TypeId,
    /// Type after substitution, such as `u32`.
    pub(crate) substituted_tid: TypeId,
}

/// Associated type projection normalized to an impl-provided value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProjectionNormalization {
    /// Type occurrence whose value is the projection result.
    pub(crate) projection_tid: TypeId,
    /// Self type for the projection.
    pub(crate) self_tid: TypeId,
    /// Associated type member used for normalization.
    pub(crate) assoc_type: DefId,
    /// Trait type that provides the associated type member.
    pub(crate) trait_tid: TypeId,
    /// Type assigned by the matching impl item.
    pub(crate) value_tid: TypeId,
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
