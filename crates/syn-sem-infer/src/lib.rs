//! Type inference for `syn-sem`.
//!
//! This crate exercises `syn-sem-pr` and `syn-sem-name` as an upper semantic phase before
//! `syn-sem-pr` V2 requirements are finalized. Its current public surface lowers represented type
//! occurrences into inference-oriented type facts while preserving path syntax and keeping name
//! lookup classification separate from final type solving.

mod types;

pub use types::*;
