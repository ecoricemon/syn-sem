//! Shared `logic-eval` substrate used by inference phases.
//!
//! Domain-specific rules live with their owning phase, such as `projection::term`. This module
//! keeps only common atom encoding, equality rules, solver symbols, and small term utilities.

mod atom;
mod equality;
pub(crate) mod symbol;
mod visit;

pub(crate) use atom::{
    atom, def_id, def_id_from_term, expr_id, term, type_id, type_id_from_term, Atom, Clause, Expr,
    Term,
};
pub(crate) use equality::{same_type, same_type_rules, type_equal_clause, SameTypeRules};
pub(crate) use visit::visit_left_var;
