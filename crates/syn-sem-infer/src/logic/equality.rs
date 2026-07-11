//! Shared equality rules and predicates for inference logic.

use super::{
    atom::{atom, term, Clause, Expr, Term},
    symbol::{Rel, Var},
};

/// Builds reflexive and structural-class equality rules.
pub(crate) fn same_type_rules() -> [Clause<'static>; 2] {
    [
        Clause::fact(same_type(atom(Var::Type), atom(Var::Type))),
        Clause::rule(
            same_type(atom(Var::Left), atom(Var::Right)),
            Expr::And(vec![
                Expr::Term(type_class(atom(Var::Left), atom(Var::Class))),
                Expr::Term(type_class(atom(Var::Right), atom(Var::Class))),
            ]),
        ),
    ]
}

/// Creates a fact assigning one inference type to its structural class.
pub(crate) fn type_class_clause<'cx>(ty: Term<'cx>, class: Term<'cx>) -> Clause<'cx> {
    Clause::fact(type_class(ty, class))
}

pub(crate) fn same_type<'cx>(left: Term<'cx>, right: Term<'cx>) -> Term<'cx> {
    term(Rel::SameType, vec![left, right])
}

fn type_class<'cx>(ty: Term<'cx>, class: Term<'cx>) -> Term<'cx> {
    term(Rel::TypeClass, vec![ty, class])
}
