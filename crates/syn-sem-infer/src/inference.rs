//! Inference phases that populate and refine [`InferDb`](crate::InferDb).
//!
//! Declaration inference handles source type occurrences that are already explicit in
//! `syn-sem-hir`. Body inference consumes HIR-owned lowered body facts, while expression typing can
//! grow separately without mixing with declaration-type lowering.

mod body;
mod decl;
mod expr;

pub(crate) use decl::analyze;
