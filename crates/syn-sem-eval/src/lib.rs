//! Constant evaluation for `syn-sem`.
//!
//! This crate consumes HIR from `syn-sem-hir`, name facts from `syn-sem-name`, and type facts from
//! `syn-sem-infer`. It owns evaluated constant facts while `syn-sem-top` remains responsible for
//! deciding when to alternate evaluation with inference.
//!
//! Evaluation distinguishes three outcomes. A known value is stored in [`EvalDb`], an unavailable
//! value remains absent, and reaching a constant expression shape that the evaluator does not
//! support returns an error. Unavailable values include unresolved inputs, cyclic constant
//! dependencies, arithmetic failure, and values that cannot yet be represented with the available
//! inference facts. Runtime-only expressions outside an [`EvalPlan`] target are not evaluated.

mod db;
mod plan;
mod required;
mod value;

pub use db::*;
pub use plan::*;
pub use value::*;
