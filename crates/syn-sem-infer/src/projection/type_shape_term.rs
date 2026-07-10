//! Logic terms for structural inference type shapes.

use crate::{
    logic::{
        atom, def_id, expr_id,
        symbol::{Ctor, Rel, Var},
        term, type_id, Atom, Clause, Term,
    },
    PrimitiveType, TypeId,
};
use syn_sem_common::CommonCx;
use syn_sem_hir as hir;
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
    ty: TypeId,
    mode: TypeShapeMode,
    shape: Term<'cx>,
) -> Clause<'cx> {
    Clause::fact(type_shape(type_id(ty), type_shape_mode(mode), shape))
}

/// * ty - Type id being described
/// * mode - `#preserve_generics` or `#variable_generics`
/// * shape - Structured type term
///
/// # Examples
///
/// * Output - `#type_shape(ty0, #preserve_generics, #primitive(u32))`
pub(crate) fn type_shape<'cx>(ty: Term<'cx>, mode: Term<'cx>, shape: Term<'cx>) -> Term<'cx> {
    term(Rel::TypeShape, vec![ty, mode, shape])
}

pub(crate) fn type_shape_mode(mode: TypeShapeMode) -> Term<'static> {
    let functor = match mode {
        TypeShapeMode::PreserveGenerics => Ctor::PreserveGenerics,
        TypeShapeMode::VariableGenerics => Ctor::VariableGenerics,
    };
    atom(functor)
}

pub(crate) fn shape_array<'cx>(elem: Term<'cx>, len: Term<'cx>) -> Term<'cx> {
    term(Ctor::Array, vec![elem, len])
}

pub(crate) fn shape_infer(ty: TypeId) -> Term<'static> {
    term(Ctor::Infer, vec![type_id(ty)])
}

pub(crate) fn shape_primitive(primitive: PrimitiveType) -> Term<'static> {
    term(Ctor::Primitive, vec![atom(primitive)])
}

pub(crate) fn shape_path<'cx>(def: DefId, args: Vec<Term<'cx>>) -> Term<'cx> {
    term(Ctor::Path, vec![shape_def(def), shape_arg(args)])
}

pub(crate) fn shape_generic_param(def: DefId) -> Term<'static> {
    term(Ctor::GenericParam, vec![shape_def(def)])
}

pub(crate) fn shape_reference<'cx>(elem: Term<'cx>, is_mut: bool) -> Term<'cx> {
    if is_mut {
        term(Ctor::Ref, vec![term(Ctor::Mut, vec![elem])])
    } else {
        term(Ctor::Ref, vec![elem])
    }
}

pub(crate) fn shape_slice<'cx>(elem: Term<'cx>) -> Term<'cx> {
    term(Ctor::Slice, vec![elem])
}

pub(crate) fn shape_tuple<'cx>(elems: Vec<Term<'cx>>) -> Term<'cx> {
    term(Ctor::Tuple, elems)
}

pub(crate) fn shape_assoc_type_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    ty: Term<'cx>,
) -> Term<'cx> {
    term(Ctor::AssocTypeArg, vec![shape_name(ccx, name), ty])
}

pub(crate) fn shape_assoc_const_arg<'cx>(
    ccx: &'cx CommonCx,
    name: &str,
    value: Term<'cx>,
) -> Term<'cx> {
    term(Ctor::AssocConstArg, vec![shape_name(ccx, name), value])
}

pub(crate) fn shape_const_int<'cx>(ccx: &'cx CommonCx, value: &str) -> Term<'cx> {
    term(Ctor::ConstInt, vec![atom(Atom::Int(ccx.intern(value)))])
}

pub(crate) fn shape_const_float<'cx>(ccx: &'cx CommonCx, value: &str) -> Term<'cx> {
    term(Ctor::ConstFloat, vec![atom(Atom::Float(ccx.intern(value)))])
}

pub(crate) fn shape_const_bool(value: bool) -> Term<'static> {
    term(Ctor::ConstBool, vec![atom(Atom::Bool(value))])
}

pub(crate) fn shape_len_expr(expr: hir::ExprId) -> Term<'static> {
    term(Ctor::LenExpr, vec![expr_id(expr)])
}

pub(crate) fn shape_len_const_usize(value: usize) -> Term<'static> {
    term(
        Ctor::LenConst,
        vec![term(Ctor::ConstUsize, vec![atom(Atom::Usize(value))])],
    )
}

pub(crate) fn shape_name<'cx>(ccx: &'cx CommonCx, name: &str) -> Term<'cx> {
    term(Ctor::Name, vec![atom(Atom::Text(ccx.intern(name)))])
}

pub(crate) fn shape_generic_var(def: DefId) -> Term<'static> {
    atom(Var::GenericParam(def))
}

fn shape_def(def: DefId) -> Term<'static> {
    term(Ctor::Def, vec![def_id(def)])
}

fn shape_arg<'cx>(args: Vec<Term<'cx>>) -> Term<'cx> {
    term(Ctor::Arg, args)
}
