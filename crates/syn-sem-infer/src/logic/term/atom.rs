//! Shared atom and identifier encoding helpers for `logic-eval` terms.

use crate::{TypeId, TypeSubject};
use logic_eval::{Clause, Term};
use syn_sem_common::{CommonCx, InternedStr};
use syn_sem_name::DefId;

// In examples below, `tyN` encodes `TypeId::new(N)`, `defN` encodes `DefId::new(N)`,
// and `exprN` encodes `syn_sem_hir::ExprId::new(N)`.

/// * ty_id - Inference type id to encode as a logic atom
///
/// # Examples
///
/// * Input - `ty_id = TypeId::new(0)`
/// * Output - `ty0`
pub(in crate::logic) fn type_id<'cx>(ccx: &'cx CommonCx, ty_id: TypeId) -> LogicTerm<'cx> {
    let value = format!("ty{}", ty_id.index());
    atom(ccx, &value)
}

/// Decodes an inference type id from a logic term.
pub(in crate::logic) fn type_id_from_term(term: &LogicTerm<'_>) -> Option<TypeId> {
    let value = term.functor.as_ref();
    let index = value.strip_prefix("ty")?.parse().ok()?;
    if !term.args.is_empty() {
        return None;
    }
    Some(TypeId::new(index))
}

/// def-id atom.
///
/// * id - Name definition id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = DefId::new(2)`
/// * Output - `def2`
pub(in crate::logic) fn def_id<'cx>(ccx: &'cx CommonCx, id: DefId) -> LogicTerm<'cx> {
    let value = format!("def{}", id.index());
    atom(ccx, &value)
}

/// Encodes a body-local type subject as a logic atom.
pub(in crate::logic) fn type_subject<'cx>(
    ccx: &'cx CommonCx,
    subject: TypeSubject,
) -> LogicTerm<'cx> {
    match subject {
        TypeSubject::Def(def) => def_id(ccx, def),
        TypeSubject::Expr(expr) => expr_id(ccx, expr),
        TypeSubject::Type(ty_id) => type_id(ccx, ty_id),
    }
}

/// * name - Logic variable name without `$`
///
/// # Examples
///
/// * Input - `name = "Self"`
/// * Output - `$Self`
pub(in crate::logic) fn var<'cx>(ccx: &'cx CommonCx, name: &str) -> LogicTerm<'cx> {
    let value = format!("${name}");
    atom(ccx, &value)
}

/// Creates a zero-argument logic atom.
pub(in crate::logic) fn atom<'cx>(ccx: &'cx CommonCx, functor: &str) -> LogicTerm<'cx> {
    term(ccx, functor, Vec::new())
}

/// Creates a logic term with a functor and arguments.
pub(in crate::logic) fn term<'cx>(
    ccx: &'cx CommonCx,
    functor: &str,
    args: Vec<LogicTerm<'cx>>,
) -> LogicTerm<'cx> {
    Term {
        functor: ccx.intern(functor),
        args,
    }
}

/// expr-id atom.
///
/// * id - HIR expression id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = ExprId::new(1)`
/// * Output - `expr1`
fn expr_id<'cx>(ccx: &'cx CommonCx, id: syn_sem_hir::ExprId) -> LogicTerm<'cx> {
    let value = format!("expr{}", id.index());
    atom(ccx, &value)
}

pub(in crate::logic) type LogicAtom<'cx> = InternedStr<'cx>;
pub(in crate::logic) type LogicTerm<'cx> = Term<LogicAtom<'cx>>;
pub(in crate::logic) type LogicClause<'cx> = Clause<LogicAtom<'cx>>;
