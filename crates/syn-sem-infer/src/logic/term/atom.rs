//! Shared atom and identifier encoding helpers for `logic-eval` terms.

use crate::{TypeId, TypeSubject};
use logic_eval::{Clause, Term, VAR_PREFIX};
use std::fmt::{self, Display};
use syn_sem_common::{CommonCx, InternedStr};
use syn_sem_name::DefId;

const TYPE_ID_PREFIX: &str = "ty";
const DEF_ID_PREFIX: &str = "def";
const EXPR_ID_PREFIX: &str = "expr";

// In examples below, `tyN` encodes `TypeId::new(N)`, `defN` encodes `DefId::new(N)`,
// and `exprN` encodes `syn_sem_hir::ExprId::new(N)`.

/// * ty_id - Inference type id to encode as a logic atom
///
/// # Examples
///
/// * Input - `ty_id = TypeId::new(0)`
/// * Output - `ty0`
pub(in crate::logic) fn type_id<'cx>(ccx: &'cx CommonCx, ty_id: TypeId) -> LogicTerm<'cx> {
    let functor = intern_prefixed_number(ccx, TYPE_ID_PREFIX, ty_id.index());
    atom(functor)
}

/// Decodes an inference type id from a logic term.
pub(in crate::logic) fn type_id_from_term(term: &LogicTerm<'_>) -> Option<TypeId> {
    let functor = term.functor.as_ref();
    let index = functor.strip_prefix(TYPE_ID_PREFIX)?.parse().ok()?;
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
    let functor = intern_prefixed_number(ccx, DEF_ID_PREFIX, id.index());
    atom(functor)
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
    let functor = intern_prefixed_number(ccx, EXPR_ID_PREFIX, id.index());
    atom(functor)
}

/// Encodes an inference subject as a logic atom.
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

/// * name - Logic variable name starting with `$`
///
/// # Examples
///
/// * Input - `name = "$Self"`
/// * Output - `$Self`
pub(in crate::logic) fn var<'cx>(functor: LogicAtom<'cx>) -> LogicTerm<'cx> {
    assert!(functor.starts_with(VAR_PREFIX));
    atom(functor)
}

/// Creates a zero-argument logic atom.
pub(in crate::logic) fn atom<'cx>(functor: LogicAtom<'cx>) -> LogicTerm<'cx> {
    term(functor, Vec::new())
}

/// Creates a logic term with a functor and arguments.
pub(in crate::logic) fn term<'cx>(
    functor: LogicAtom<'cx>,
    args: Vec<LogicTerm<'cx>>,
) -> LogicTerm<'cx> {
    Term { functor, args }
}

fn intern_prefixed_number<'cx>(ccx: &'cx CommonCx, prefix: &str, number: usize) -> LogicAtom<'cx> {
    struct PrefixedNumber<'a> {
        prefix: &'a str,
        number: usize,
    }

    impl Display for PrefixedNumber<'_> {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str(self.prefix)?;
            Display::fmt(&self.number, f)
        }
    }

    let len = prefix.len() + number.checked_ilog10().unwrap_or(0) as usize + 1;
    ccx.intern_display(&PrefixedNumber { prefix, number }, len)
        .unwrap()
}

pub(in crate::logic) type LogicAtom<'cx> = InternedStr<'cx>;
pub(in crate::logic) type LogicTerm<'cx> = Term<LogicAtom<'cx>>;
pub(in crate::logic) type LogicClause<'cx> = Clause<LogicAtom<'cx>>;
