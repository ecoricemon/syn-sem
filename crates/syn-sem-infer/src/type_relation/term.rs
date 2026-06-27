//! Logic rules and predicates for type relation resolution.

use crate::{
    logic::{
        same_type, same_type_rules,
        symbol::{pred, var},
        type_equal_clause as raw_type_equal_clause, type_id, type_subject, CreateTerm, LogicAtom,
        LogicClause, LogicTerm, SameTypeRules,
    },
    TypeEqualityFact, TypeId, TypeSubject,
};
use logic_eval::{Clause, Expr};
use syn_sem_common::CommonCx;

/// Type relation resolution follows collected equality edges, needs reverse and transitive closure,
/// and does not need global reflexivity.
const TYPE_RELATION_SAME_TYPE_RULES: SameTypeRules = SameTypeRules {
    reflexive: false,
    reverse: true,
    transitive: true,
};

/// * Rule 0 - `#same_type(A, B) :- #type_equal(A, B).`
/// * Rule 1 - `#same_type(A, B) :- #type_equal(B, A).`
/// * Rule 2 - `#same_type(A, C) :- #same_type(A, B), #same_type(B, C).`
/// * Rule 3 - `#resolved_type(Subject, Type) :- #same_type(Subject, Type), #type_candidate(Type).`
///
/// # Examples
///
/// * Input facts - `#type_equal(def0, expr1).`, `#type_equal(def0, ty2).`,
///   `#type_candidate(ty2).`
/// * Query - `#resolved_type(expr1, $Type).`
/// * Output - `$Type = ty2`
pub(crate) fn type_relation_rules<'cx>(ccx: &'cx CommonCx) -> Vec<LogicClause<'cx>> {
    let mut rules = same_type_rules(ccx, TYPE_RELATION_SAME_TYPE_RULES);
    rules.push(Clause {
        head: resolved_type(ccx, ccx.atom(var::SUBJECT), ccx.atom(var::TYPE)),
        body: Some(Expr::And(vec![
            Expr::Term(same_type(ccx, ccx.atom(var::SUBJECT), ccx.atom(var::TYPE))),
            Expr::Term(type_candidate(ccx, ccx.atom(var::TYPE))),
        ])),
    });
    rules
}

/// * fact - One type relation equality edge
///
/// # Examples
///
/// * Input - `left = def0`, `right = expr1`
/// * Output - `#type_equal(def0, expr1).`
pub(crate) fn type_equality_clause<'cx>(
    ccx: &'cx CommonCx,
    fact: TypeEqualityFact,
) -> LogicClause<'cx> {
    raw_type_equal_clause(
        ccx,
        type_subject(ccx, fact.left),
        type_subject(ccx, fact.right),
    )
}

/// * ty - Inference type candidate known to type relation logic
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `#type_candidate(ty0).`
pub(crate) fn type_candidate_clause<'cx>(ccx: &'cx CommonCx, ty: TypeId) -> LogicClause<'cx> {
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
/// * Output - `Expr::Term(#resolved_type(expr0, $Type))`
pub(crate) fn resolved_type_query<'cx>(
    ccx: &'cx CommonCx,
    subject: TypeSubject,
) -> Expr<LogicAtom<'cx>> {
    Expr::Term(resolved_type(
        ccx,
        type_subject(ccx, subject),
        ccx.atom(var::TYPE),
    ))
}

/// * subject - Subject with a resolved type
///
/// # Examples
///
/// * Input - `subject = expr0`, `ty = ty1`
/// * Output - `#resolved_type(expr0, ty1)`
fn resolved_type<'cx>(
    ccx: &'cx CommonCx,
    subject: LogicTerm<'cx>,
    ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(pred::RESOLVED_TYPE, vec![subject, ty])
}

/// * ty - Concrete inference type
///
/// # Examples
///
/// * Input - `ty = ty0`
/// * Output - `#type_candidate(ty0)`
fn type_candidate<'cx>(ccx: &'cx CommonCx, ty: LogicTerm<'cx>) -> LogicTerm<'cx> {
    ccx.term(pred::TYPE_CANDIDATE, vec![ty])
}
