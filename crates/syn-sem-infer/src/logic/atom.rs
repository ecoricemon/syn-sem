//! Shared atom and identifier encoding helpers for `logic-eval` terms.

use super::symbol;
use crate::{TypeId, TypeSubject};
use logic_eval::{Clause, Term};
use syn_sem_common::{intern_prefixed_number, CommonCx, InternedStr};
use syn_sem_name::DefId;

// In examples below, `tyN` encodes `TypeId::new(N)`, `defN` encodes `DefId::new(N)`,
// and `exprN` encodes `syn_sem_hir::ExprId::new(N)`.

/// * ty - Inference type id to encode as a logic atom
///
/// # Examples
///
/// * Input - `ty = TypeId::new(0)`
/// * Output - `ty0`
pub(crate) fn type_id<'cx>(ccx: &'cx CommonCx, ty: TypeId) -> LogicTerm<'cx> {
    let functor = intern_prefixed_number(ccx, symbol::id::TYPE, ty.index());
    atom(functor)
}

/// Decodes an inference type id from a logic term.
pub(crate) fn type_id_from_term(term: &LogicTerm<'_>) -> Option<TypeId> {
    let functor = term.functor.as_ref();
    let index = functor.strip_prefix(symbol::id::TYPE)?.parse().ok()?;
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
pub(crate) fn def_id<'cx>(ccx: &'cx CommonCx, id: DefId) -> LogicTerm<'cx> {
    let functor = intern_prefixed_number(ccx, symbol::id::DEF, id.index());
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
pub(crate) fn expr_id<'cx>(ccx: &'cx CommonCx, id: syn_sem_hir::ExprId) -> LogicTerm<'cx> {
    let functor = intern_prefixed_number(ccx, symbol::id::EXPR, id.index());
    atom(functor)
}

/// Encodes an inference subject as a logic atom.
pub(crate) fn type_subject<'cx>(ccx: &'cx CommonCx, subject: TypeSubject) -> LogicTerm<'cx> {
    match subject {
        TypeSubject::Def(def) => def_id(ccx, def),
        TypeSubject::Expr(expr) => expr_id(ccx, expr),
        TypeSubject::Type(ty) => type_id(ccx, ty),
    }
}

/// Creates a zero-argument logic atom.
pub(crate) fn atom<'cx>(functor: LogicAtom<'cx>) -> LogicTerm<'cx> {
    term(functor, Vec::new())
}

/// Creates a logic term with a functor and arguments.
pub(crate) fn term<'cx>(functor: LogicAtom<'cx>, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
    Term { functor, args }
}

pub(crate) trait CreateTerm<Func> {
    fn atom(&self, functor: Func) -> LogicTerm<'_>;
    fn term<'a>(&'a self, functor: Func, args: Vec<LogicTerm<'a>>) -> LogicTerm<'a>;
}

impl CreateTerm<&str> for CommonCx {
    fn atom(&self, functor: &str) -> LogicTerm<'_> {
        atom(self.intern(functor))
    }

    fn term<'a>(&'a self, functor: &str, args: Vec<LogicTerm<'a>>) -> LogicTerm<'a> {
        term(self.intern(functor), args)
    }
}

pub(crate) type LogicAtom<'cx> = InternedStr<'cx>;
pub(crate) type LogicTerm<'cx> = Term<LogicAtom<'cx>>;
pub(crate) type LogicClause<'cx> = Clause<LogicAtom<'cx>>;
