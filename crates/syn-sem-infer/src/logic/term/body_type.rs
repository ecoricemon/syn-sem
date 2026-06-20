//! Logic rules and predicates for body-local type equality resolution.

use super::atom::{term, type_id, type_subject, var, LogicAtom, LogicClause, LogicTerm};
use crate::{TypeEqualFact, TypeId, TypeSubject};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;

const PRED_RESOLVED_TYPE: &str = "resolved_type";
const PRED_SAME_TYPE: &str = "same_type";
const PRED_TYPE_CANDIDATE: &str = "type_candidate";
const PRED_TYPE_EQUAL: &str = "type_equal";

const VAR_ARG: &str = "Arg";
const VAR_SUBJECT: &str = "Subject";
const VAR_TYPE: &str = "Type";

/// * Rule 0 - `same_type(A, B) :- type_equal(A, B).`
/// * Rule 1 - `same_type(A, B) :- type_equal(B, A).`
/// * Rule 2 - `same_type(A, C) :- type_equal(A, B), same_type(B, C).`
/// * Rule 3 - `same_type(A, C) :- type_equal(B, A), same_type(B, C).`
/// * Rule 4 - `resolved_type(Subject, Type) :- same_type(Subject, Type), type_candidate(Type).`
///
/// # Examples
///
/// * Input facts - `type_equal(def0, expr1).`, `type_equal(def0, ty2).`,
///   `type_candidate(ty2).`
/// * Query - `resolved_type(expr1, $Type).`
/// * Output - `$Type = ty2`
pub(in crate::logic) fn body_type_rules<'cx>(ccx: &'cx CommonCx) -> [LogicClause<'cx>; 5] {
    [
        Clause {
            head: same_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE)),
            body: Some(Expr::Term(type_equal(
                ccx,
                var(ccx, VAR_SUBJECT),
                var(ccx, VAR_TYPE),
            ))),
        },
        Clause {
            head: same_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE)),
            body: Some(Expr::Term(type_equal(
                ccx,
                var(ccx, VAR_TYPE),
                var(ccx, VAR_SUBJECT),
            ))),
        },
        Clause {
            head: same_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE)),
            body: Some(Expr::And(vec![
                Expr::Term(type_equal(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_ARG))),
                Expr::Term(same_type(ccx, var(ccx, VAR_ARG), var(ccx, VAR_TYPE))),
            ])),
        },
        Clause {
            head: same_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE)),
            body: Some(Expr::And(vec![
                Expr::Term(type_equal(ccx, var(ccx, VAR_ARG), var(ccx, VAR_SUBJECT))),
                Expr::Term(same_type(ccx, var(ccx, VAR_ARG), var(ccx, VAR_TYPE))),
            ])),
        },
        Clause {
            head: resolved_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE)),
            body: Some(Expr::And(vec![
                Expr::Term(same_type(ccx, var(ccx, VAR_SUBJECT), var(ccx, VAR_TYPE))),
                Expr::Term(type_candidate(ccx, var(ccx, VAR_TYPE))),
            ])),
        },
    ]
}

/// * fact - One body-local type equality edge
///
/// # Examples
///
/// * Input - `left = def0`, `right = expr1`
/// * Output - `type_equal(def0, expr1).`
pub(in crate::logic) fn type_equal_clause<'cx>(
    ccx: &'cx CommonCx,
    fact: TypeEqualFact,
) -> LogicClause<'cx> {
    Clause {
        head: type_equal(
            ccx,
            type_subject(ccx, fact.left),
            type_subject(ccx, fact.right),
        ),
        body: None,
    }
}

/// * ty - Inference type candidate known to body type logic
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `type_candidate(ty0).`
pub(in crate::logic) fn type_candidate_clause<'cx>(
    ccx: &'cx CommonCx,
    ty_id: TypeId,
) -> LogicClause<'cx> {
    Clause {
        head: type_candidate(ccx, type_id(ccx, ty_id)),
        body: None,
    }
}

/// * subject - Body-local subject whose type candidates are requested
///
/// # Examples
///
/// * Input - `subject = expr0`
/// * Output - `Expr::Term(resolved_type(expr0, $Type))`
pub(in crate::logic) fn resolved_type_query<'cx>(
    ccx: &'cx CommonCx,
    subject: TypeSubject,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(resolved_type(
        ccx,
        type_subject(ccx, subject),
        var(ccx, VAR_TYPE),
    ))
}

/// * subject - Subject with a body-local type
///
/// # Examples
///
/// * Input - `subject = expr0`, `ty = ty1`
/// * Output - `resolved_type(expr0, ty1)`
fn resolved_type<'cx>(
    ccx: &'cx CommonCx,
    subject: LogicTerm<'cx>,
    ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx, PRED_RESOLVED_TYPE, vec![subject, ty])
}

/// * ty - Concrete inference type
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `type_candidate(ty0)`
fn type_candidate<'cx>(ccx: &'cx CommonCx, ty: LogicTerm<'cx>) -> LogicTerm<'cx> {
    term(ccx, PRED_TYPE_CANDIDATE, vec![ty])
}

/// * left - One body-local type subject
/// * right - Another body-local type subject
///
/// # Examples
///
/// * Input - `left = def0`, `right = expr1`
/// * Output - `type_equal(def0, expr1)`
fn type_equal<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx, PRED_TYPE_EQUAL, vec![left, right])
}

/// * left - One body-local type subject
/// * right - Another body-local type subject
///
/// # Examples
///
/// * Input - `left = expr0`, `right = ty1`
/// * Output - `same_type(expr0, ty1)`
fn same_type<'cx>(
    ccx: &'cx CommonCx,
    left: LogicTerm<'cx>,
    right: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx, PRED_SAME_TYPE, vec![left, right])
}
