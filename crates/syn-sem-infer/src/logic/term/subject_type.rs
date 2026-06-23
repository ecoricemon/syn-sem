//! Logic rules and predicates for subject type equality resolution.

use super::{
    atom::{term, type_id, type_subject, var, LogicAtom, LogicClause, LogicTerm},
    equality::{same_type, same_type_rules, SameTypeRules},
};
use crate::{TypeEqualFact, TypeId, TypeSubject};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;

const PRED_RESOLVED_TYPE: &str = "resolved_type";
const PRED_TYPE_CANDIDATE: &str = "type_candidate";

const VAR_SUBJECT: &str = "$Subject";
pub(crate) const VAR_TYPE: &str = "$Type";

/// Subject type resolution follows collected equality edges, needs reverse and transitive closure,
/// and does not need global reflexivity.
const SUBJECT_TYPE_SAME_TYPE_RULES: SameTypeRules = SameTypeRules {
    reflexive: false,
    reverse: true,
    transitive: true,
};

/// * Rule 0 - `same_type(A, B) :- type_equal(A, B).`
/// * Rule 1 - `same_type(A, B) :- type_equal(B, A).`
/// * Rule 2 - `same_type(A, C) :- same_type(A, B), same_type(B, C).`
/// * Rule 3 - `resolved_type(Subject, Type) :- same_type(Subject, Type), type_candidate(Type).`
///
/// # Examples
///
/// * Input facts - `type_equal(def0, expr1).`, `type_equal(def0, ty2).`,
///   `type_candidate(ty2).`
/// * Query - `resolved_type(expr1, $Type).`
/// * Output - `$Type = ty2`
pub(in crate::logic) fn subject_type_rules<'cx>(ccx: &'cx CommonCx) -> Vec<LogicClause<'cx>> {
    let mut rules = same_type_rules(ccx, SUBJECT_TYPE_SAME_TYPE_RULES);
    rules.push(Clause {
        head: resolved_type(ccx, var(ccx.intern(VAR_SUBJECT)), var(ccx.intern(VAR_TYPE))),
        body: Some(Expr::And(vec![
            Expr::Term(same_type(
                ccx,
                var(ccx.intern(VAR_SUBJECT)),
                var(ccx.intern(VAR_TYPE)),
            )),
            Expr::Term(type_candidate(ccx, var(ccx.intern(VAR_TYPE)))),
        ])),
    });
    rules
}

/// * fact - One subject type equality edge
///
/// # Examples
///
/// * Input - `left = def0`, `right = expr1`
/// * Output - `type_equal(def0, expr1).`
pub(in crate::logic) fn type_equal_clause<'cx>(
    ccx: &'cx CommonCx,
    fact: TypeEqualFact,
) -> LogicClause<'cx> {
    super::equality::type_equal_clause(
        ccx,
        type_subject(ccx, fact.left),
        type_subject(ccx, fact.right),
    )
}

/// * ty - Inference type candidate known to subject type logic
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `type_candidate(ty0).`
pub(in crate::logic) fn type_candidate_clause<'cx>(
    ccx: &'cx CommonCx,
    ty: TypeId,
) -> LogicClause<'cx> {
    Clause {
        head: type_candidate(ccx, type_id(ccx, ty)),
        body: None,
    }
}

/// * subject - Subject whose type candidates are requested
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
        var(ccx.intern(VAR_TYPE)),
    ))
}

/// * subject - Subject with a resolved type
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
    term(ccx.intern(PRED_RESOLVED_TYPE), vec![subject, ty])
}

/// * ty - Concrete inference type
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `type_candidate(ty0)`
fn type_candidate<'cx>(ccx: &'cx CommonCx, ty: LogicTerm<'cx>) -> LogicTerm<'cx> {
    term(ccx.intern(PRED_TYPE_CANDIDATE), vec![ty])
}
