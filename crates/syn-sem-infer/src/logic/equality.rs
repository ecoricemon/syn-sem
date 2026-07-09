//! Shared equality rules and predicates for inference logic.

use super::{
    atom::{atom, term, Clause, Expr, Term},
    symbol::{Rel, Var},
};

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
pub(crate) fn same_type_rules(rules: SameTypeRules) -> Vec<Clause<'static>> {
    let mut clauses = vec![Clause::rule(
        same_type(atom(Var::Left), atom(Var::Right)),
        Expr::Term(type_equal(atom(Var::Left), atom(Var::Right))),
    )];

    if rules.reflexive {
        clauses.push(Clause::fact(same_type(atom(Var::Type), atom(Var::Type))));
    }

    if rules.reverse {
        clauses.push(Clause::rule(
            same_type(atom(Var::Left), atom(Var::Right)),
            Expr::Term(type_equal(atom(Var::Right), atom(Var::Left))),
        ));
    }

    if rules.transitive {
        clauses.push(Clause::rule(
            same_type(atom(Var::Left), atom(Var::Type)),
            // Keep the recursive terms in this order for `logic-eval`'s query-time tabling.
            Expr::And(vec![
                Expr::Term(same_type(atom(Var::Arg), atom(Var::Type))),
                Expr::Term(same_type(atom(Var::Left), atom(Var::Arg))),
            ]),
        ));
    }

    clauses
}

/// Creates a direct equality fact between two logic terms.
pub(crate) fn type_equal_clause<'cx>(left: Term<'cx>, right: Term<'cx>) -> Clause<'cx> {
    Clause::fact(type_equal(left, right))
}

pub(crate) fn same_type<'cx>(left: Term<'cx>, right: Term<'cx>) -> Term<'cx> {
    term(Rel::SameType, vec![left, right])
}

fn type_equal<'cx>(left: Term<'cx>, right: Term<'cx>) -> Term<'cx> {
    term(Rel::TypeEqual, vec![left, right])
}
