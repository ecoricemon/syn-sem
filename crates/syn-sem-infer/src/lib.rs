//! Type inference for `syn-sem`.
//!
//! This crate consumes the current HIR container from `syn-sem-hir` and name facts from
//! `syn-sem-name`. Its current public surface lowers HIR type occurrences into
//! inference-owned type ids while preserving path syntax and keeping name lookup classification
//! separate from final type solving.
//!
//! Upper phases should enter through [`InferDb::analyze`], ask for a type occurrence with
//! [`InferDb::type_for_hir_type`] or [`InferDb::normalized_type_for_hir_type`], and inspect the
//! resulting [`Type`] through focused query methods such as [`InferDb::projection`] and
//! [`InferDb::projection_normalization`].

mod db;
mod id;
mod inference;
mod logic;
mod obligations;
mod types;

pub use db::*;
pub use id::*;
pub(crate) use obligations::*;
pub use types::*;
