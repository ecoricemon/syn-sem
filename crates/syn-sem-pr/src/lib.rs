//! Program representation for `syn-sem`.
//!
//! This crate owns the current Rust source program representation. It preserves declaration,
//! type, and block entry structure in stable arenas while leaving future desugared body IR as an
//! extension.
//!
//! In concrete terms, this crate:
//!
//! - Builds the representation from `syn-sem-ast` inputs.
//! - Hides direct AST traversal from upper semantic phases.
//! - Preserves Rust source declaration, type, block, and expression-entry structure.
//! - Structures that source shape through stable id-based arenas.
//! - Stores connection ids such as `DefId` and `ScopeId`, while leaving resolved name, import,
//!   visibility, and type facts to other semantic phases.

mod id;
mod repr;

pub use id::*;
pub use repr::*;
