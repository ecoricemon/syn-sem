//! Shared equality rules and predicates for inference logic.

use super::atom::{CreateTerm, LogicClause, LogicTerm};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;

use super::symbol::{pred, var};

/// Configures which closure rules are added over `type_equal` facts.
///
/// The base rule `#same_type(A, B) :- #type_equal(A, B).` is always included.
#[derive(Clone, Copy)]
pub(crate) struct SameTypeRules {
    /// Adds a general `same_type(A, A).` rule.
    ///
    /// Domains that need reflexivity over a known finite set can instead insert ground
    /// `type_equal(A, A).` facts and use the base rule.
    pub(crate) reflexive: bool,
    /// Adds `same_type(A, B) :- type_equal(B, A).`.
    ///
    /// Together with the base rule, this makes `type_equal` facts symmetric.
    pub(crate) reverse: bool,
    /// Adds `same_type(A, C) :- same_type(A, B), same_type(B, C).`.
    pub(crate) transitive: bool,
}

/// * `#same_type($Left, $Right) :- #type_equal($Left, $Right).`
/// * `#same_type($Type, $Type).`
/// * `#same_type($Left, $Right) :- #type_equal($Right, $Left).`
pub(crate) fn same_type_rules<'cx>(
    ccx: &'cx CommonCx,
    rules: SameTypeRules,
) -> Vec<LogicClause<'cx>> {
    let mut clauses = vec![Clause {
        head: same_type(ccx, ccx.atom(var::LEFT), ccx.atom(var::RIGHT)),
        body: Some(Expr::Term(type_equal(
            ccx,
            ccx.atom(var::LEFT),
            ccx.atom(var::RIGHT),
        ))),
    }];

    if rules.reflexive {
        clauses.push(Clause {
            head: same_type(ccx, ccx.atom(var::TYPE), ccx.atom(var::TYPE)),
            body: None,
        });
    }

    if rules.reverse {
        clauses.push(Clause {
            head: same_type(ccx, ccx.atom(var::LEFT), ccx.atom(var::RIGHT)),
            body: Some(Expr::Term(type_equal(
                ccx,
                ccx.atom(var::RIGHT),
                ccx.atom(var::LEFT),
            ))),
        });
    }

    if rules.transitive {
        clauses.push(Clause {
            head: same_type(ccx, ccx.atom(var::LEFT), ccx.atom(var::TYPE)),
            // Keep the recursive terms in this order for `logic-eval`'s query-time tabling.
            body: Some(Expr::And(vec![
                Expr::Term(same_type(ccx, ccx.atom(var::ARG), ccx.atom(var::TYPE))),
                Expr::Term(same_type(ccx, ccx.atom(var::LEFT), ccx.atom(var::ARG))),
            ])),
        });
    }

    clauses
}

/// Creates a direct equality fact between two logic terms.
pub(crate) fn type_equal_clause<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicClause<'cx> {
    Clause {
        head: type_equal(ccx, left, right),
        body: None,
    }
}

pub(crate) fn same_type<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(pred::SAME_TYPE, vec![left, right])
}

fn type_equal<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(pred::TYPE_EQUAL, vec![left, right])
}
