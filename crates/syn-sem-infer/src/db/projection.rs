//! Projection-specific facts and query helpers for inference.

use crate::{
    PathTypeResolution, ProjectionCandidate, ProjectionMatch, ProjectionNormalization,
    ProjectionObligation, ProjectionType, TypeId,
};

/// Associated type projection facts owned by inference.
#[derive(Debug, Default)]
pub struct ProjectionDb {
    pub(crate) obligations: Vec<ProjectionObligation>,
    pub(crate) candidates: Vec<ProjectionCandidate>,
    pub(crate) matches: Vec<ProjectionMatch>,
    pub(crate) normalizations: Vec<ProjectionNormalization>,
}

impl ProjectionDb {
    /// Returns associated type projections that still need solver work.
    #[cfg(test)]
    pub(crate) fn obligations(&self) -> &[ProjectionObligation] {
        &self.obligations
    }

    /// Returns projection candidates derived from obligations and known trait bounds.
    #[cfg(test)]
    pub(crate) fn candidates(&self) -> &[ProjectionCandidate] {
        &self.candidates
    }

    /// Returns projections matched against concrete associated type members.
    #[cfg(test)]
    pub(crate) fn matches(&self) -> &[ProjectionMatch] {
        &self.matches
    }

    /// Returns projections normalized to impl-provided value types.
    #[cfg(test)]
    pub(crate) fn normalizations(&self) -> &[ProjectionNormalization] {
        &self.normalizations
    }

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
