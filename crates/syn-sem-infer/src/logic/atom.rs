//! Shared atom and identifier encoding helpers for `logic-eval` terms.

use super::symbol::{Ctor, Rel, Var};
use crate::{PrimitiveType, TypeClassId, TypeId};
use syn_sem_common::Str;
use syn_sem_hir as hir;
use syn_sem_name::DefId;

pub(crate) type Term<'cx> = logic_eval::Term<Atom<'cx>>;
pub(crate) type Expr<'cx> = logic_eval::Expr<Atom<'cx>>;
pub(crate) type Clause<'cx> = logic_eval::Clause<Atom<'cx>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum Atom<'cx> {
    Var(Var),
    Rel(Rel),
    Ctor(Ctor),
    Ty(TypeId),
    TyClass(TypeClassId),
    Def(DefId),
    Expr(hir::ExprId),
    Prim(PrimitiveType),
    Text(Str<'cx>),
    Int(Str<'cx>),
    Float(Str<'cx>),
    Bool(bool),
    Usize(usize),
}

impl logic_eval::Atom for Atom<'_> {
    fn is_variable(&self) -> bool {
        matches!(self, Self::Var(_))
    }
}

impl From<Var> for Atom<'_> {
    fn from(value: Var) -> Self {
        Self::Var(value)
    }
}

impl From<Rel> for Atom<'_> {
    fn from(value: Rel) -> Self {
        Self::Rel(value)
    }
}

impl From<Ctor> for Atom<'_> {
    fn from(value: Ctor) -> Self {
        Self::Ctor(value)
    }
}

impl From<TypeId> for Atom<'_> {
    fn from(value: TypeId) -> Self {
        Self::Ty(value)
    }
}

impl From<TypeClassId> for Atom<'_> {
    fn from(value: TypeClassId) -> Self {
        Self::TyClass(value)
    }
}

impl From<DefId> for Atom<'_> {
    fn from(value: DefId) -> Self {
        Self::Def(value)
    }
}

impl From<hir::ExprId> for Atom<'_> {
    fn from(value: hir::ExprId) -> Self {
        Self::Expr(value)
    }
}

impl From<PrimitiveType> for Atom<'_> {
    fn from(value: PrimitiveType) -> Self {
        Self::Prim(value)
    }
}

impl<'cx> From<Str<'cx>> for Atom<'cx> {
    fn from(value: Str<'cx>) -> Self {
        Self::Text(value)
    }
}

/// * ty - Inference type id to encode as a logic atom
///
/// # Examples
///
/// * Input - `ty = TypeId::new(0)`
/// * Output - `Atom::Ty(TypeId::new(0))`
pub(crate) fn type_id(ty: TypeId) -> Term<'static> {
    atom(ty)
}

/// Encodes a structural inference-type class as a logic atom.
pub(crate) fn type_class_id(class: TypeClassId) -> Term<'static> {
    atom(class)
}

/// Decodes an inference type id from a logic term.
pub(crate) fn type_id_from_term(term: &Term<'_>) -> Option<TypeId> {
    match (term.functor, term.args.as_slice()) {
        (Atom::Ty(ty), []) => Some(ty),
        _ => None,
    }
}

/// def-id atom.
///
/// * id - Name definition id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = DefId::new(2)`
/// * Output - `Atom::Def(DefId::new(2))`
pub(crate) fn def_id(id: DefId) -> Term<'static> {
    atom(id)
}

/// Decodes a name definition id from a logic term.
pub(crate) fn def_id_from_term(term: &Term<'_>) -> Option<DefId> {
    match (term.functor, term.args.as_slice()) {
        (Atom::Def(def), []) => Some(def),
        _ => None,
    }
}

/// expr-id atom.
///
/// * id - HIR expression id to encode as a logic atom
///
/// # Examples
///
/// * Input - `id = ExprId::new(1)`
/// * Output - `Atom::Expr(ExprId::new(1))`
pub(crate) fn expr_id(id: hir::ExprId) -> Term<'static> {
    atom(id)
}

/// Creates a zero-argument logic atom.
pub(crate) fn atom<'cx>(functor: impl Into<Atom<'cx>>) -> Term<'cx> {
    term(functor, Vec::new())
}

/// Creates a logic term with a functor and arguments.
pub(crate) fn term<'cx>(functor: impl Into<Atom<'cx>>, args: Vec<Term<'cx>>) -> Term<'cx> {
    Term {
        functor: functor.into(),
        args,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use logic_eval::Atom as _;

    #[test]
    fn classifies_typed_variables_without_name_prefixes() {
        assert!(Atom::Var(Var::Trait).is_variable());
        assert!(Atom::Var(Var::GenericParam(DefId::new(0))).is_variable());
        assert!(!Atom::Ty(TypeId::new(0)).is_variable());
        assert!(!Atom::Rel(Rel::SameType).is_variable());
    }

    #[test]
    fn decodes_ids_from_typed_atoms() {
        let ty = TypeId::new(3);
        let def = DefId::new(5);

        assert_eq!(type_id_from_term(&atom(ty)), Some(ty));
        assert_eq!(def_id_from_term(&atom(def)), Some(def));
        assert_eq!(type_id_from_term(&atom(def)), None);
        assert_eq!(def_id_from_term(&atom(ty)), None);
    }
}
