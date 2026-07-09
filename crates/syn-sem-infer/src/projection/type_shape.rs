//! Logic shape encoding for inference types.

use super::type_shape_term::{self as term, TypeShapeMode};
use crate::{
    logic::Term, ArrayLen, ConstArg, GenericArg, InferConstFacts, InferConstValue, InferTypes, Lit,
    PathType, PathTypeResolution, Type, TypeId,
};
use syn_sem_common::{CommonCx, Map};

pub(crate) struct TypeShape<'cx> {
    pub(crate) term: Term<'cx>,
    /// Type ids recoverable from subterms emitted while building `shape`.
    ///
    /// Logic answers can bind shape variables to structural terms such as `#primitive(u32)` or
    /// `#generic_param(#def(U))`. This map keeps the bridge back to the inference arena so answer
    /// terms can be interpreted as type ids. If one shape contains the same term more than once,
    /// the first type id is used as the representative.
    ///
    /// Keep this map attached to one encoded shape. The same structural term can appear while
    /// encoding different root types, and a root-level map would lose which shape produced that
    /// term. Within one shape, duplicate terms are equivalent for shape matching, so choosing one
    /// representative type id is enough to materialize the matched type component.
    pub(crate) term_types: Map<Term<'cx>, TypeId>,
}

pub(crate) struct TypeShapeEncoder<'a, 'cx> {
    ccx: &'cx CommonCx,
    types: &'a InferTypes<'cx>,
    const_facts: &'a InferConstFacts,
}

impl<'a, 'cx> TypeShapeEncoder<'a, 'cx> {
    pub(crate) fn new(
        ccx: &'cx CommonCx,
        types: &'a InferTypes<'cx>,
        const_facts: &'a InferConstFacts,
    ) -> Self {
        Self {
            ccx,
            types,
            const_facts,
        }
    }

    /// Encodes a type as the structural term used by `#type_shape`.
    ///
    /// # [`TypeShapeMode::PreserveGenerics`]
    ///
    /// Generic parameters stay as syntax-facing type components instead of becoming logic
    /// variables.
    ///
    /// For example, given this Rust code will be encoded:
    /// ```text
    /// <Vec<u32> as Iterator>::Item ▸ #path(#def(Vec), #arg(#primitive(u32)))
    /// struct S<U: Identity> {
    ///     f: <Vec<U> as Identity>::Output, ▸ #path(#def(Vec), #arg(#generic_param(#def(U))))
    /// }
    /// ```
    ///
    /// # [`TypeShapeMode::VariableGenerics`]
    ///
    /// Generic parameters from an impl self type become logic variables so they can match concrete
    /// projection self components.
    ///
    /// For example, given this Rust code will be encoded:
    /// ```text
    /// impl<T> Iterator for Vec<T> { ▸ #path(#def(Vec), #arg($G_T))
    ///     type Item = T;
    /// }
    /// ```
    pub(crate) fn encode(&self, ty: TypeId, mode: TypeShapeMode) -> Option<TypeShape<'cx>> {
        let mut term_types = Map::default();
        let term = self.type_term(ty, mode, &mut term_types)?;
        Some(TypeShape { term, term_types })
    }

    fn type_term(
        &self,
        ty: TypeId,
        mode: TypeShapeMode,
        term_types: &mut Map<Term<'cx>, TypeId>,
    ) -> Option<Term<'cx>> {
        if mode == TypeShapeMode::VariableGenerics {
            if let Some(def) = self.types.generic_def(ty) {
                let term = term::shape_generic_var(def);
                term_types.entry(term.clone()).or_insert(ty);
                return Some(term);
            }
        }

        let term = match &self.types[ty] {
            Type::Array { elem, len } => Some(term::shape_array(
                self.type_term(*elem, mode, term_types)?,
                Self::array_len_term(*len),
            )),
            Type::Infer => Some(term::shape_infer(ty)),
            Type::Primitive(primitive) => Some(term::shape_primitive(*primitive)),
            Type::Path(path) => self.path_term(path, mode, term_types),
            Type::Reference { elem, is_mut } => {
                let elem = self.type_term(*elem, mode, term_types)?;
                Some(term::shape_reference(elem, *is_mut))
            }
            Type::Slice { elem } => {
                Some(term::shape_slice(self.type_term(*elem, mode, term_types)?))
            }
            Type::Tuple { elems } => {
                let elems = elems
                    .iter()
                    .map(|elem| self.type_term(*elem, mode, term_types))
                    .collect::<Option<Vec<_>>>()?;
                Some(term::shape_tuple(elems))
            }
        }?;

        if mode == TypeShapeMode::PreserveGenerics {
            term_types.entry(term.clone()).or_insert(ty);
        }
        Some(term)
    }

    fn path_term(
        &self,
        path: &PathType<'cx>,
        mode: TypeShapeMode,
        term_types: &mut Map<Term<'cx>, TypeId>,
    ) -> Option<Term<'cx>> {
        let def = match &path.resolution {
            PathTypeResolution::GenericParam(def) if mode == TypeShapeMode::PreserveGenerics => {
                return Some(term::shape_generic_param(*def));
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
            .map(|arg| self.generic_arg_term(arg, mode, term_types))
            .collect::<Option<Vec<_>>>()?;
        Some(term::shape_path(def, args))
    }

    fn generic_arg_term(
        &self,
        arg: &GenericArg<'cx>,
        mode: TypeShapeMode,
        term_types: &mut Map<Term<'cx>, TypeId>,
    ) -> Option<Term<'cx>> {
        match arg {
            GenericArg::Type(ty) => self.type_term(*ty, mode, term_types),
            GenericArg::Const(arg) => self.const_arg_term(arg),
            GenericArg::AssocType { name, ty } => Some(term::shape_assoc_type_arg(
                self.ccx,
                name.as_ref(),
                self.type_term(*ty, mode, term_types)?,
            )),
            GenericArg::AssocConst { name, value } => Some(term::shape_assoc_const_arg(
                self.ccx,
                name.as_ref(),
                self.const_arg_term(value)?,
            )),
            GenericArg::Constraint { .. } | GenericArg::Unsupported => None,
        }
    }

    fn const_arg_term(&self, arg: &ConstArg<'cx>) -> Option<Term<'cx>> {
        match arg {
            ConstArg::Lit(Lit::Int(value)) => Some(term::shape_const_int(self.ccx, value.as_ref())),
            ConstArg::Lit(Lit::Float(value)) => {
                Some(term::shape_const_float(self.ccx, value.as_ref()))
            }
            ConstArg::Lit(Lit::Bool(value)) => Some(term::shape_const_bool(*value)),
            ConstArg::Expr(expr) => self
                .const_facts
                .const_expr_value(*expr)
                .map(|value| self.const_value_term(value)),
            ConstArg::Path { def: Some(def), .. } => self
                .const_facts
                .const_def_value(*def)
                .map(|value| self.const_value_term(value)),
            ConstArg::Path { def: None, .. } => None,
        }
    }

    fn const_value_term(&self, value: InferConstValue) -> Term<'cx> {
        match value {
            InferConstValue::Int(value) => {
                term::shape_const_int(self.ccx, &value.value.to_string())
            }
            InferConstValue::Bool(value) => term::shape_const_bool(value),
        }
    }

    fn array_len_term(len: ArrayLen) -> Term<'cx> {
        match len {
            ArrayLen::Expr(expr) => term::shape_len_expr(expr),
            ArrayLen::ConstUsize(value) => term::shape_len_const_usize(value),
        }
    }
}
