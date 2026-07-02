//! Equality relations between type subjects and their resolved type mappings.
//!
//! A type subject is a definition, HIR expression occurrence, or already lowered inference type.
//! The collector records equality edges such as `let y = x` or `expr == annotated type`, the
//! resolver follows those edges to concrete type candidates, and expression-specific derivation can
//! add new equality facts that feed the next fixed-point iteration.

mod collect;
mod db;
mod expr;
mod pat;
mod resolve;

#[cfg(test)]
mod tests;

pub(crate) use collect::TypeRelationCollector;
pub(crate) use db::{ResolvedTypeFact, TypeEqualityFact, TypeRelationDb, TypeSubject};
pub(crate) use expr::ExprTypeDeriver;
pub(crate) use pat::PatTypeDeriver;
pub(crate) use resolve::TypeRelationResolver;
