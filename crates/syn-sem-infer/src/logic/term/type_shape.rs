//! Logic terms for structural inference type shapes.

use super::{
    atom::{term, type_id, LogicClause, LogicTerm},
    symbol::{func, pred},
};
use crate::TypeId;
use logic_eval::Clause;
use syn_sem_common::CommonCx;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::logic) enum TypeShapeMode {
    Concrete,
    ImplPattern,
}

/// * ty - Type id whose stored type is exposed as a structured logic term
/// * mode - Whether generics are encoded as concrete parameters or impl-pattern variables
/// * shape - Structured type term, such as `#path(#def(Vec), #arg($G_T))`
///
/// # Examples
///
/// * Output - `#type_shape(ty0, #impl_pattern, #path(#def(def1), #arg($G2))).`
pub(in crate::logic) fn type_shape_clause<'cx>(
    ccx: &'cx CommonCx,
    ty: TypeId,
    mode: TypeShapeMode,
    shape: LogicTerm<'cx>,
) -> LogicClause<'cx> {
    Clause {
        head: type_shape(ccx, type_id(ccx, ty), type_shape_mode(ccx, mode), shape),
        body: None,
    }
}

/// * ty - Type id being described
/// * mode - `#concrete` or `#impl_pattern`
/// * shape - Structured type term
///
/// # Examples
///
/// * Output - `#type_shape(ty0, #concrete, #primitive(u32))`
pub(in crate::logic) fn type_shape<'cx>(
    ccx: &'cx CommonCx,
    ty: LogicTerm<'cx>,
    mode: LogicTerm<'cx>,
    shape: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    term(ccx.intern(pred::TYPE_SHAPE), vec![ty, mode, shape])
}

pub(in crate::logic) fn type_shape_mode<'cx>(
    ccx: &'cx CommonCx,
    mode: TypeShapeMode,
) -> LogicTerm<'cx> {
    let functor = match mode {
        TypeShapeMode::Concrete => func::CONCRETE,
        TypeShapeMode::ImplPattern => func::IMPL_PATTERN,
    };
    term(ccx.intern(functor), Vec::new())
}
