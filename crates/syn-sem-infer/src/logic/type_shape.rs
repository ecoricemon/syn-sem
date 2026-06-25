//! Logic shape encoding for inference types.

use crate::{
    logic::term::{self, symbol::func},
    ArrayLen, ConstArg, GenericArg, InferTypes, Lit, PathType, PathTypeResolution, Type, TypeId,
};
use logic_eval::Term;
use syn_sem_common::{intern_prefixed_number, CommonCx};
use syn_sem_name::DefId;

type LogicTerm<'cx> = Term<term::LogicAtom<'cx>>;

pub(in crate::logic) struct TypeShape<'cx> {
    pub(in crate::logic) shape: LogicTerm<'cx>,
    pub(in crate::logic) generic_vars: Vec<(term::LogicAtom<'cx>, TypeId)>,
    pub(in crate::logic) concrete_terms: Vec<(LogicTerm<'cx>, TypeId)>,
}

pub(in crate::logic) struct TypeShapeEncoder<'a, 'cx> {
    ccx: &'cx CommonCx,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> TypeShapeEncoder<'a, 'cx> {
    pub(in crate::logic) fn new(ccx: &'cx CommonCx, types: &'a InferTypes<'cx>) -> Self {
        Self { ccx, types }
    }

    pub(in crate::logic) fn encode(
        &self,
        ty: TypeId,
        mode: term::TypeShapeMode,
    ) -> Option<TypeShape<'cx>> {
        let mut generic_vars = Vec::new();
        let mut concrete_terms = Vec::new();
        let shape = self.type_term(ty, mode, &mut generic_vars, &mut concrete_terms)?;
        Some(TypeShape {
            shape,
            generic_vars,
            concrete_terms,
        })
    }

    fn type_term(
        &self,
        ty: TypeId,
        mode: term::TypeShapeMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        if mode == term::TypeShapeMode::ImplPattern {
            if let Some(def) = Self::generic_def(self.types, ty) {
                return Some(Self::generic_var(self.ccx, def, ty, generic_vars));
            }
        }

        let term = match &self.types[ty] {
            Type::Array { elem, len } => Some(self.logic_term(
                func::ARRAY,
                vec![
                    self.type_term(*elem, mode, generic_vars, concrete_terms)?,
                    self.array_len_term(*len),
                ],
            )),
            Type::Infer => Some(self.logic_term(func::INFER, vec![self.type_id_term(ty)])),
            Type::Primitive(primitive) => Some(self.primitive_term(*primitive)),
            Type::Path(path) => self.path_term(path, mode, generic_vars, concrete_terms),
            Type::Reference { elem, is_mut } => {
                let elem = self.type_term(*elem, mode, generic_vars, concrete_terms)?;
                if *is_mut {
                    Some(self.logic_term(func::REF, vec![self.logic_term(func::MUT, vec![elem])]))
                } else {
                    Some(self.logic_term(func::REF, vec![elem]))
                }
            }
            Type::Slice { elem } => Some(self.logic_term(
                func::SLICE,
                vec![self.type_term(*elem, mode, generic_vars, concrete_terms)?],
            )),
            Type::Tuple { elems } => {
                let elems = elems
                    .iter()
                    .map(|elem| self.type_term(*elem, mode, generic_vars, concrete_terms))
                    .collect::<Option<Vec<_>>>()?;
                Some(self.logic_term(func::TUPLE, elems))
            }
        }?;

        if mode == term::TypeShapeMode::Concrete
            && concrete_terms
                .iter()
                .all(|(candidate, _)| candidate != &term)
        {
            concrete_terms.push((term.clone(), ty));
        }
        Some(term)
    }

    fn path_term(
        &self,
        path: &PathType<'cx>,
        mode: term::TypeShapeMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        let def = match &path.resolution {
            PathTypeResolution::GenericParam(def) if mode == term::TypeShapeMode::Concrete => {
                return Some(self.logic_term(
                    func::GENERIC_PARAM,
                    vec![self.logic_term(func::DEF, vec![self.def_id_term(*def)])],
                ));
            }
            PathTypeResolution::Nominal(def) => *def,
            PathTypeResolution::GenericParam(_)
            | PathTypeResolution::Projection(_)
            | PathTypeResolution::Ambiguous(_)
            | PathTypeResolution::Unresolved => return None,
        };
        let args = path
            .path
            .segments
            .iter()
            .flat_map(|segment| &segment.args)
            .map(|arg| self.generic_arg_term(arg, mode, generic_vars, concrete_terms))
            .collect::<Option<Vec<_>>>()?;
        Some(self.logic_term(
            func::PATH,
            vec![
                self.logic_term(func::DEF, vec![self.def_id_term(def)]),
                self.logic_term(func::ARG, args),
            ],
        ))
    }

    fn generic_arg_term(
        &self,
        arg: &GenericArg<'cx>,
        mode: term::TypeShapeMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        match arg {
            GenericArg::Type(ty) => self.type_term(*ty, mode, generic_vars, concrete_terms),
            GenericArg::Const(arg) => self.const_arg_term(arg),
            GenericArg::AssocType { name, ty } => Some(self.logic_term(
                func::ASSOC_TYPE_ARG,
                vec![
                    self.name_term(name.as_ref()),
                    self.type_term(*ty, mode, generic_vars, concrete_terms)?,
                ],
            )),
            GenericArg::AssocConst { name, value } => Some(self.logic_term(
                func::ASSOC_CONST_ARG,
                vec![self.name_term(name.as_ref()), self.const_arg_term(value)?],
            )),
            GenericArg::Constraint { .. } | GenericArg::Unsupported => None,
        }
    }

    fn const_arg_term(&self, arg: &ConstArg<'cx>) -> Option<LogicTerm<'cx>> {
        match arg {
            ConstArg::Lit(Lit::Int(value)) => Some(self.logic_term(
                func::CONST_INT,
                vec![self.logic_term(value.as_ref(), Vec::new())],
            )),
            ConstArg::Lit(Lit::Float(value)) => Some(self.logic_term(
                func::CONST_FLOAT,
                vec![self.logic_term(value.as_ref(), Vec::new())],
            )),
            ConstArg::Lit(Lit::Bool(value)) => Some(self.logic_term(
                func::CONST_BOOL,
                vec![self.logic_term(if *value { "true" } else { "false" }, Vec::new())],
            )),
            ConstArg::Path(_) | ConstArg::Expr(_) => None,
        }
    }

    fn array_len_term(&self, len: ArrayLen) -> LogicTerm<'cx> {
        match len {
            ArrayLen::Expr(expr) => self.logic_term(func::LEN_EXPR, vec![self.expr_id_term(expr)]),
        }
    }

    fn generic_var(
        ccx: &'cx CommonCx,
        def: DefId,
        generic: TypeId,
        vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
    ) -> LogicTerm<'cx> {
        let var = Self::prefixed_number_atom(ccx, "$G", def.index());
        if vars.iter().all(|(candidate, _)| *candidate != var) {
            vars.push((var, generic));
        }
        Term {
            functor: var,
            args: Vec::new(),
        }
    }

    fn type_id_term(&self, ty: TypeId) -> LogicTerm<'cx> {
        self.prefixed_number_term("ty", ty.index())
    }

    fn def_id_term(&self, def: DefId) -> LogicTerm<'cx> {
        self.prefixed_number_term("def", def.index())
    }

    fn expr_id_term(&self, expr: syn_sem_hir::ExprId) -> LogicTerm<'cx> {
        self.prefixed_number_term("expr", expr.index())
    }

    fn primitive_term(&self, primitive: crate::PrimitiveType) -> LogicTerm<'cx> {
        self.logic_term(
            func::PRIMITIVE,
            vec![self.logic_term(Self::primitive_name(primitive), Vec::new())],
        )
    }

    fn name_term(&self, name: &str) -> LogicTerm<'cx> {
        self.logic_term(func::NAME, vec![self.logic_term(name, Vec::new())])
    }

    fn primitive_name(primitive: crate::PrimitiveType) -> &'static str {
        match primitive {
            crate::PrimitiveType::AbstractInt => "abstract_int",
            crate::PrimitiveType::AbstractFloat => "abstract_float",
            crate::PrimitiveType::Bool => "bool",
            crate::PrimitiveType::Char => "char",
            crate::PrimitiveType::Str => "str",
            crate::PrimitiveType::I8 => "i8",
            crate::PrimitiveType::I16 => "i16",
            crate::PrimitiveType::I32 => "i32",
            crate::PrimitiveType::I64 => "i64",
            crate::PrimitiveType::I128 => "i128",
            crate::PrimitiveType::Isize => "isize",
            crate::PrimitiveType::U8 => "u8",
            crate::PrimitiveType::U16 => "u16",
            crate::PrimitiveType::U32 => "u32",
            crate::PrimitiveType::U64 => "u64",
            crate::PrimitiveType::U128 => "u128",
            crate::PrimitiveType::Usize => "usize",
            crate::PrimitiveType::F32 => "f32",
            crate::PrimitiveType::F64 => "f64",
        }
    }

    fn prefixed_number_term(&self, prefix: &str, number: usize) -> LogicTerm<'cx> {
        Term {
            functor: Self::prefixed_number_atom(self.ccx, prefix, number),
            args: Vec::new(),
        }
    }

    fn prefixed_number_atom(
        ccx: &'cx CommonCx,
        prefix: &str,
        number: usize,
    ) -> term::LogicAtom<'cx> {
        intern_prefixed_number(ccx, prefix, number)
    }

    fn logic_term(&self, functor: &str, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
        Term {
            functor: self.ccx.intern(functor),
            args,
        }
    }

    fn generic_def(types: &InferTypes<'cx>, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &types[ty] else {
            return None;
        };
        let PathTypeResolution::GenericParam(def) = path.resolution else {
            return None;
        };
        Some(def)
    }
}
