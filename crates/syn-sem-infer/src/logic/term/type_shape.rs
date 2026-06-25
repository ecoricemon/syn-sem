//! Logic terms for structural inference type shapes.

use super::{
    atom::{atom, def_id, expr_id, type_id, CreateTerm, LogicClause, LogicTerm},
    symbol::{func, pred},
};
use crate::{PrimitiveType, TypeId};
use logic_eval::Clause;
use syn_sem_common::{intern_prefixed_number, CommonCx};
use syn_sem_name::DefId;

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
    ccx.term(pred::TYPE_SHAPE, vec![ty, mode, shape])
}

pub(in crate::logic) fn type_shape_mode<'cx>(
    ccx: &'cx CommonCx,
    mode: TypeShapeMode,
) -> LogicTerm<'cx> {
    let functor = match mode {
        TypeShapeMode::Concrete => func::CONCRETE,
        TypeShapeMode::ImplPattern => func::IMPL_PATTERN,
    };
    ccx.atom(functor)
}

pub(in crate::logic) fn shape_array<'cx>(
    ccx: &'cx CommonCx,
    elem: LogicTerm<'cx>,
    len: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ARRAY, vec![elem, len])
}

pub(in crate::logic) fn shape_infer<'cx>(ccx: &'cx CommonCx, ty: TypeId) -> LogicTerm<'cx> {
    ccx.term(func::INFER, vec![type_id(ccx, ty)])
}

pub(in crate::logic) fn shape_primitive<'cx>(
    ccx: &'cx CommonCx,
    primitive: PrimitiveType,
) -> LogicTerm<'cx> {
    ccx.term(func::PRIMITIVE, vec![ccx.atom(primitive.name())])
}

pub(in crate::logic) fn shape_path<'cx>(
    ccx: &'cx CommonCx,
    def: DefId,
    args: Vec<LogicTerm<'cx>>,
) -> LogicTerm<'cx> {
    ccx.term(func::PATH, vec![shape_def(ccx, def), shape_arg(ccx, args)])
}

pub(in crate::logic) fn shape_generic_param<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    ccx.term(func::GENERIC_PARAM, vec![shape_def(ccx, def)])
}

pub(in crate::logic) fn shape_reference<'cx>(
    ccx: &'cx CommonCx,
    elem: LogicTerm<'cx>,
    is_mut: bool,
) -> LogicTerm<'cx> {
    if is_mut {
        ccx.term(func::REF, vec![ccx.term(func::MUT, vec![elem])])
    } else {
        ccx.term(func::REF, vec![elem])
    }
}

pub(in crate::logic) fn shape_slice<'cx>(
    ccx: &'cx CommonCx,
    elem: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::SLICE, vec![elem])
}

pub(in crate::logic) fn shape_tuple<'cx>(
    ccx: &'cx CommonCx,
    elems: Vec<LogicTerm<'cx>>,
) -> LogicTerm<'cx> {
    ccx.term(func::TUPLE, elems)
}

pub(in crate::logic) fn shape_assoc_type_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ASSOC_TYPE_ARG, vec![shape_name(ccx, name), ty])
}

pub(in crate::logic) fn shape_assoc_const_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    value: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ASSOC_CONST_ARG, vec![shape_name(ccx, name), value])
}

pub(in crate::logic) fn shape_const_int<'cx>(ccx: &'cx CommonCx, value: &str) -> LogicTerm<'cx> {
    ccx.term(func::CONST_INT, vec![ccx.atom(value)])
}

pub(in crate::logic) fn shape_const_float<'cx>(ccx: &'cx CommonCx, value: &str) -> LogicTerm<'cx> {
    ccx.term(func::CONST_FLOAT, vec![ccx.atom(value)])
}

pub(in crate::logic) fn shape_const_bool<'cx>(ccx: &'cx CommonCx, value: bool) -> LogicTerm<'cx> {
    ccx.term(
        func::CONST_BOOL,
        vec![ccx.atom(if value { "true" } else { "false" })],
    )
}

pub(in crate::logic) fn shape_len_expr<'cx>(
    ccx: &'cx CommonCx,
    expr: syn_sem_hir::ExprId,
) -> LogicTerm<'cx> {
    ccx.term(func::LEN_EXPR, vec![expr_id(ccx, expr)])
}

pub(in crate::logic) fn shape_name<'cx>(ccx: &'cx CommonCx, name: &str) -> LogicTerm<'cx> {
    ccx.term(func::NAME, vec![ccx.atom(name)])
}

pub(in crate::logic) fn shape_generic_var<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    atom(intern_prefixed_number(ccx, "$G", def.index()))
}

fn shape_def<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    ccx.term(func::DEF, vec![def_id(ccx, def)])
}

fn shape_arg<'cx>(ccx: &'cx CommonCx, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
    ccx.term(func::ARG, args)
}
