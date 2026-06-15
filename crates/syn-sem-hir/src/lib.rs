//! HIR construction for `syn-sem`.
//!
//! This crate owns the upper-phase HIR layer: build HIR from `syn-sem-ast` inputs plus
//! `syn-sem-name` facts, then expose source spine and lowered semantic input to later phases.
//!
//! In concrete terms, this crate:
//!
//! - Builds HIR from `syn-sem-ast` inputs and name data.
//! - Hides direct AST traversal from upper semantic phases.
//! - Preserves Rust source declaration, type, block, statement, local binding, pattern, and
//!   expression spine through stable id-based arenas.
//! - Lowers semantic inputs such as generic predicates for inference-friendly consumers.
//! - Stores connection ids such as `DefId` and `ScopeId`, while leaving resolved name, import,
//!   visibility, and type facts to other semantic phases.

mod builder;
mod hir;
mod id;
mod lower;

pub use builder::HirBuilder;
pub use hir::*;
pub use id::*;
