//! HIR lowering layers for upper semantic phases.
//!
//! This module exposes lowered facts derived from the source-shaped HIR while keeping resolved
//! name, type, and evaluation decisions in their owning semantic phases.

mod generic_predicate;
mod lowered_blocks;

pub(crate) use generic_predicate::*;
pub use lowered_blocks::*;
