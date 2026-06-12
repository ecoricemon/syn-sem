//! Inference phases that populate and refine [`InferDb`](crate::InferDb).
//!
//! Declaration inference handles source type occurrences that are already explicit in
//! `syn-sem-pr`. Body and expression inference are separate modules so block-level orchestration
//! and expression typing can grow without mixing with declaration-type lowering.

mod body;
mod decl;
mod expr;

pub(crate) use decl::analyze;
