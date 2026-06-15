//! Temporary top-level orchestration crate for `syn-sem`.
//!
//! This crate wires extracted phases together while the old `syn-sem` facade is being migrated.
//! It currently provides parsing through `syn-sem-ast` and name collection through
//! `syn-sem-name`.

mod context;
mod semantics;

pub use context::*;
pub use semantics::*;
