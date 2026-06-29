//! Associated type projection collection, matching, and normalization.
//!
//! This module owns the projection work for types such as `<T as Iterator>::Item` and `T::Assoc`.
//! The collector records projection obligations from lowered type paths, the logic adapter matches
//! those obligations against trait bounds and impl associated-type facts, and the normalizer
//! records normalizations from a projection type to the impl-provided value type.

mod collect;
mod db;
mod logic;
mod normalize;
mod term;
mod type_shape;
mod type_shape_term;

pub(crate) use collect::ProjectionCollector;
pub use db::ProjectionNormalizationResult;
pub(crate) use db::{
    ImplSelfGenericBinding, ImplSelfMatch, ProjectionDb, ProjectionMatch, ProjectionNormalization,
    ProjectionObligation, ProjectionTypeSubstitution,
};
pub(crate) use normalize::ProjectionNormalizer;
pub(crate) use type_shape::{TypeShape, TypeShapeEncoder};
