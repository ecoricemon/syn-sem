//! Temporary top-level orchestration crate for `syn-sem`.
//!
//! This crate wires extracted phases together while the old `syn-sem` facade is being migrated.
//! It currently provides parsing, name collection, type inference, and constant evaluation.

mod context;
mod semantics;

pub use context::*;
pub use semantics::*;
