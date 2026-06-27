//! Logic terms for structural inference type shapes.

use crate::{
    logic::{
        atom, def_id, expr_id,
        symbol::{func, pred},
        type_id, CreateTerm, LogicClause, LogicTerm,
    },
    PrimitiveType, TypeId,
};
use logic_eval::Clause;
use syn_sem_common::{intern_prefixed_number, CommonCx};
use syn_sem_name::DefId;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeShapeMode {
    PreserveGenerics,
    VariableGenerics,
}

/// * ty - Type id whose stored type is exposed as a structured logic term
/// * mode - Whether generic parameters are preserved as terms or encoded as variables
/// * shape - Structured type term, such as `#path(#def(Vec), #arg($G_T))`
///
/// # Examples
///
/// * Output - `#type_shape(ty0, #variable_generics, #path(#def(def1), #arg($G2))).`
pub(crate) fn type_shape_clause<'cx>(
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
/// * mode - `#preserve_generics` or `#variable_generics`
/// * shape - Structured type term
///
/// # Examples
///
/// * Output - `#type_shape(ty0, #preserve_generics, #primitive(u32))`
pub(crate) fn type_shape<'cx>(
    ccx: &'cx CommonCx,
    ty: LogicTerm<'cx>,
    mode: LogicTerm<'cx>,
    shape: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(pred::TYPE_SHAPE, vec![ty, mode, shape])
}

pub(crate) fn type_shape_mode<'cx>(ccx: &'cx CommonCx, mode: TypeShapeMode) -> LogicTerm<'cx> {
    let functor = match mode {
        TypeShapeMode::PreserveGenerics => func::PRESERVE_GENERICS,
        TypeShapeMode::VariableGenerics => func::VARIABLE_GENERICS,
    };
    ccx.atom(functor)
}

pub(crate) fn shape_array<'cx>(
    ccx: &'cx CommonCx,
    elem: LogicTerm<'cx>,
    len: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ARRAY, vec![elem, len])
}

pub(crate) fn shape_infer<'cx>(ccx: &'cx CommonCx, ty: TypeId) -> LogicTerm<'cx> {
    ccx.term(func::INFER, vec![type_id(ccx, ty)])
}

pub(crate) fn shape_primitive<'cx>(ccx: &'cx CommonCx, primitive: PrimitiveType) -> LogicTerm<'cx> {
    ccx.term(func::PRIMITIVE, vec![ccx.atom(primitive.name())])
}

pub(crate) fn shape_path<'cx>(
    ccx: &'cx CommonCx,
    def: DefId,
    args: Vec<LogicTerm<'cx>>,
) -> LogicTerm<'cx> {
    ccx.term(func::PATH, vec![shape_def(ccx, def), shape_arg(ccx, args)])
}

pub(crate) fn shape_generic_param<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    ccx.term(func::GENERIC_PARAM, vec![shape_def(ccx, def)])
}

pub(crate) fn shape_reference<'cx>(
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

pub(crate) fn shape_slice<'cx>(ccx: &'cx CommonCx, elem: LogicTerm<'cx>) -> LogicTerm<'cx> {
    ccx.term(func::SLICE, vec![elem])
}

pub(crate) fn shape_tuple<'cx>(ccx: &'cx CommonCx, elems: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
    ccx.term(func::TUPLE, elems)
}

pub(crate) fn shape_assoc_type_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    ty: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ASSOC_TYPE_ARG, vec![shape_name(ccx, name), ty])
}

pub(crate) fn shape_assoc_const_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    value: LogicTerm<'cx>,
) -> LogicTerm<'cx> {
    ccx.term(func::ASSOC_CONST_ARG, vec![shape_name(ccx, name), value])
}

pub(crate) fn shape_const_int<'cx>(ccx: &'cx CommonCx, value: &str) -> LogicTerm<'cx> {
    ccx.term(func::CONST_INT, vec![ccx.atom(value)])
}

pub(crate) fn shape_const_float<'cx>(ccx: &'cx CommonCx, value: &str) -> LogicTerm<'cx> {
    ccx.term(func::CONST_FLOAT, vec![ccx.atom(value)])
}

pub(crate) fn shape_const_bool<'cx>(ccx: &'cx CommonCx, value: bool) -> LogicTerm<'cx> {
    ccx.term(
        func::CONST_BOOL,
        vec![ccx.atom(if value { "true" } else { "false" })],
    )
}

pub(crate) fn shape_len_expr<'cx>(ccx: &'cx CommonCx, expr: syn_sem_hir::ExprId) -> LogicTerm<'cx> {
    ccx.term(func::LEN_EXPR, vec![expr_id(ccx, expr)])
}

pub(crate) fn shape_len_const_usize<'cx>(ccx: &'cx CommonCx, value: usize) -> LogicTerm<'cx> {
    ccx.term(
        func::LEN_CONST,
        vec![ccx.term(
            func::CONST_USIZE,
            vec![atom(intern_prefixed_number(ccx, "usize", value))],
        )],
    )
}

pub(crate) fn shape_name<'cx>(ccx: &'cx CommonCx, name: &str) -> LogicTerm<'cx> {
    ccx.term(func::NAME, vec![ccx.atom(name)])
}

pub(crate) fn shape_generic_var<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    atom(intern_prefixed_number(ccx, "$G", def.index()))
}

fn shape_def<'cx>(ccx: &'cx CommonCx, def: DefId) -> LogicTerm<'cx> {
    ccx.term(func::DEF, vec![def_id(ccx, def)])
}

fn shape_arg<'cx>(ccx: &'cx CommonCx, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
    ccx.term(func::ARG, args)
}
