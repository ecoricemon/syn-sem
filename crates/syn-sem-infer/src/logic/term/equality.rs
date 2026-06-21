//! Shared equality rules and predicates for inference logic.

use super::atom::{term, var, LogicClause, LogicTerm};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;

const PRED_SAME_TYPE: &str = "same_type";
const PRED_TYPE_EQUAL: &str = "type_equal";

const VAR_ARG: &str = "$Arg";
const VAR_LEFT: &str = "$Left";
const VAR_RIGHT: &str = "$Right";
const VAR_TYPE: &str = "$Type";

/// Configures which closure rules are added over `type_equal` facts.
///
/// The base rule `same_type(A, B) :- type_equal(A, B).` is always included.
#[derive(Clone, Copy)]
pub(in crate::logic) struct SameTypeRules {
    /// Adds a general `same_type(A, A).` rule.
    ///
    /// Domains that need reflexivity over a known finite set can instead insert ground
    /// `type_equal(A, A).` facts and use the base rule.
    pub(in crate::logic) reflexive: bool,
    /// Adds `same_type(A, B) :- type_equal(B, A).`.
    ///
    /// Together with the base rule, this makes `type_equal` facts symmetric.
    pub(in crate::logic) reverse: bool,
    /// Adds `same_type(A, C) :- same_type(A, B), same_type(B, C).`.
    pub(in crate::logic) transitive: bool,
}

/// Returns `same_type` closure rules over `type_equal` facts.
pub(in crate::logic) fn same_type_rules<'cx>(
    ccx: &'cx CommonCx,
    rules: SameTypeRules,
) -> Vec<LogicClause<'cx>> {
    let mut clauses = vec![Clause {
        head: same_type(ccx, var(ccx.intern(VAR_LEFT)), var(ccx.intern(VAR_RIGHT))),
        body: Some(Expr::Term(type_equal(
            ccx,
            var(ccx.intern(VAR_LEFT)),
            var(ccx.intern(VAR_RIGHT)),
        ))),
    }];

    if rules.reflexive {
        clauses.push(Clause {
            head: same_type(ccx, var(ccx.intern(VAR_TYPE)), var(ccx.intern(VAR_TYPE))),
            body: None,
        });
    }

    if rules.reverse {
        clauses.push(Clause {
            head: same_type(ccx, var(ccx.intern(VAR_LEFT)), var(ccx.intern(VAR_RIGHT))),
            body: Some(Expr::Term(type_equal(
                ccx,
                var(ccx.intern(VAR_RIGHT)),
                var(ccx.intern(VAR_LEFT)),
            ))),
        });
    }

    if rules.transitive {
        clauses.push(Clause {
            head: same_type(ccx, var(ccx.intern(VAR_LEFT)), var(ccx.intern(VAR_TYPE))),
            // Keep the recursive terms in this order for `logic-eval`'s query-time tabling.
            body: Some(Expr::And(vec![
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_ARG)),
                    var(ccx.intern(VAR_TYPE)),
                )),
                Expr::Term(same_type(
                    ccx,
                    var(ccx.intern(VAR_LEFT)),
                    var(ccx.intern(VAR_ARG)),
                )),
            ])),
        });
    }

    clauses
}

/// Creates a direct equality fact between two logic terms.
pub(in crate::logic) fn type_equal_clause<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicClause<'cx> {
    Clause {
        head: type_equal(ccx, left, right),
        body: None,
    }
}

pub(in crate::logic) fn same_type<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx.intern(PRED_SAME_TYPE), vec![left, right])
}

fn type_equal<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx.intern(PRED_TYPE_EQUAL), vec![left, right])
}
