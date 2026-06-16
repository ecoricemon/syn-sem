//! HIR lowering layers for upper semantic phases.
//!
//! This module exposes lowered facts derived from the source-shaped HIR while keeping resolved
//! name, type, and evaluation decisions in their owning semantic phases.

mod body;
mod generic_predicate;

pub use body::*;
pub(crate) use generic_predicate::*;
