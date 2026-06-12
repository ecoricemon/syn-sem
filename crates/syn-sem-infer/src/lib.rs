//! Type inference for `syn-sem`.
//!
//! This crate exercises `syn-sem-pr` and `syn-sem-name` as an upper semantic phase before
//! `syn-sem-pr` V2 requirements are finalized. Its current public surface lowers represented type
//! occurrences into inference-owned type ids while preserving path syntax and keeping name lookup
//! classification separate from final type solving.
//!
//! Upper phases should enter through [`InferDb::analyze`], ask for a type occurrence with
//! [`InferDb::type_for_repr_type`] or [`InferDb::normalized_type_for_repr_type`], and inspect the
//! resulting [`Type`] through focused query methods such as [`InferDb::projection`] and
//! [`InferDb::projection_normalization`].

mod inference;
mod logic;
mod obligations;
mod types;

pub(crate) use obligations::*;
pub use types::*;
