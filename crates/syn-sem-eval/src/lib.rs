//! Constant evaluation for `syn-sem`.
//!
//! This crate consumes HIR from `syn-sem-hir`, name facts from `syn-sem-name`, and type facts from
//! `syn-sem-infer`. It owns evaluated constant facts while `syn-sem-top` remains responsible for
//! deciding when to alternate evaluation with inference.

mod db;
mod value;

pub use db::*;
pub use value::*;
