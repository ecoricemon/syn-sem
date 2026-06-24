//! Logic-backed associated type projection derivation.

use crate::{
    logic::term::{
        self,
        symbol::{func, pred, var},
    },
    ArrayLen, AssocTypeImplFact, ConstArg, GenericArg, ImplSelfMatch, ImplSelfTypeArgBinding,
    InferTypes, Lit, PathType, PathTypeResolution, ProjectionDb, ProjectionMatch,
    ProjectionNormalization, TraitBoundFact, Type, TypeId, TypeSubstitution,
};
use logic_eval::{Clause, Database, Expr, Term};
use std::fmt::{self, Display};
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

type LogicTerm<'cx> = Term<term::LogicAtom<'cx>>;

#[derive(Clone, Copy)]
enum ImplSelfTermMode {
    Concrete,
    ImplPattern,
}

/// Uses [`ProjectionLogic`] at each solver-backed step, then stores the derived projection data.
pub(crate) struct ProjectionDeriver<'a, 'cx> {
    projections: &'a mut ProjectionDb,
    types: &'a mut InferTypes<'cx>,
    ccx: &'cx CommonCx,
    trait_bound_facts: &'a [TraitBoundFact],
    assoc_type_impl_facts: &'a [AssocTypeImplFact],
    names: &'a NameDb<'cx>,
}

impl<'a, 'cx: 'a> ProjectionDeriver<'a, 'cx> {
    pub(crate) fn new(
        projections: &'a mut ProjectionDb,
        types: &'a mut InferTypes<'cx>,
        ccx: &'cx CommonCx,
        trait_bound_facts: &'a [TraitBoundFact],
        assoc_type_impl_facts: &'a [AssocTypeImplFact],
        names: &'a NameDb<'cx>,
    ) -> Self {
        Self {
            projections,
            types,
            ccx,
            trait_bound_facts,
            assoc_type_impl_facts,
            names,
        }
    }

    pub(crate) fn derive(&mut self) {
        let matches = self.derive_projection_matches();
        self.projections.matches.extend(matches);

        let (impl_self_matches, type_bindings) = self.derive_impl_self_matches();
        self.projections.impl_self_matches.extend(impl_self_matches);
        self.projections.type_bindings.extend(type_bindings);

        let substitutions = self.derive_type_substitutions();
        self.projections.type_substitutions.extend(substitutions);

        let normalizations = self.derive_projection_normalizations();
        self.projections.normalizations.extend(normalizations);
    }

    fn derive_projection_matches(&self) -> Vec<ProjectionMatch> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_projection_candidates();

        let mut matches = Vec::new();
        for obligation in &self.projections.obligations {
            let Some(self_) = obligation.self_ else {
                continue;
            };
            if let Some(trait_) = obligation.trait_ {
                if let Some(assoc) = self.trait_member_assoc(trait_, obligation.assoc) {
                    matches.push(ProjectionMatch {
                        projection: obligation.projection,
                        self_,
                        assoc,
                        trait_,
                    });
                }
                continue;
            }
            for bound in self.trait_bound_facts {
                if !logic.proves_candidate(
                    obligation.projection,
                    self_,
                    obligation.assoc,
                    bound.trait_,
                ) {
                    continue;
                }
                if let Some(assoc) = self.trait_member_assoc(bound.trait_, obligation.assoc) {
                    matches.push(ProjectionMatch {
                        projection: obligation.projection,
                        self_,
                        assoc,
                        trait_: bound.trait_,
                    });
                }
            }
        }
        matches
    }

    fn derive_impl_self_matches(&self) -> (Vec<ImplSelfMatch>, Vec<ImplSelfTypeArgBinding>) {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_impl_self_match_candidates();

        let mut impl_self_matches = Vec::new();
        let mut type_bindings = Vec::new();
        for candidate in logic.impl_self_match_candidates() {
            let Some(bindings) =
                self.logic_type_bindings(candidate.projection_self, candidate.impl_self)
            else {
                continue;
            };
            if !impl_self_matches.contains(&candidate) {
                impl_self_matches.push(candidate);
            }
            for binding in bindings {
                if !type_bindings.contains(&binding) {
                    type_bindings.push(binding);
                }
            }
        }
        (impl_self_matches, type_bindings)
    }

    fn derive_type_substitutions(&mut self) -> Vec<TypeSubstitution> {
        let mut substitutions = Vec::new();
        for impl_fact in self.assoc_type_impl_facts {
            let mut contexts = Vec::new();
            for binding in self
                .projections
                .type_bindings
                .iter()
                .filter(|binding| binding.impl_self == impl_fact.impl_self)
            {
                let context = (binding.projection_self, binding.impl_self);
                if !contexts.contains(&context) {
                    contexts.push(context);
                }
            }

            for (projection_self, impl_self) in contexts {
                let impl_bindings = self
                    .projections
                    .type_bindings
                    .iter()
                    .filter(|binding| {
                        binding.projection_self == projection_self && binding.impl_self == impl_self
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let Some((substituted, used_bindings)) =
                    Self::substitute_type(self.types, impl_fact.value_ty, &impl_bindings)
                else {
                    continue;
                };
                for binding in used_bindings {
                    let substitution = TypeSubstitution {
                        projection_self: binding.projection_self,
                        impl_self: binding.impl_self,
                        value_ty: impl_fact.value_ty,
                        generic: binding.generic,
                        arg: binding.arg,
                        substituted,
                    };
                    if !substitutions.contains(&substitution) {
                        substitutions.push(substitution);
                    }
                }
            }
        }
        substitutions
    }

    fn derive_projection_normalizations(&self) -> Vec<ProjectionNormalization> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_projection_normalizations();

        let mut normalizations = Vec::new();
        for projection_match in &self.projections.matches {
            for impl_fact in self.assoc_type_impl_facts {
                let substituted_values = self
                    .projections
                    .type_substitutions
                    .iter()
                    .filter(|substitution| {
                        substitution.projection_self == projection_match.self_
                            && substitution.impl_self == impl_fact.impl_self
                            && substitution.value_ty == impl_fact.value_ty
                    })
                    .map(|substitution| substitution.substituted);
                for value_ty in std::iter::once(impl_fact.value_ty).chain(substituted_values) {
                    if logic.proves_normalization(
                        projection_match.projection,
                        projection_match.self_,
                        projection_match.assoc,
                        projection_match.trait_,
                        value_ty,
                    ) {
                        let normalization = ProjectionNormalization {
                            projection: projection_match.projection,
                            self_: projection_match.self_,
                            assoc: projection_match.assoc,
                            trait_: projection_match.trait_,
                            value_ty,
                        };
                        if !normalizations.contains(&normalization) {
                            normalizations.push(normalization);
                        }
                    }
                }
            }
        }
        normalizations
    }

    /// Returns the associated type member in `trait_` whose name matches
    /// `requested_assoc_type`.
    ///
    /// `requested_assoc_type` is the definition found by the projection path, and is used only as
    /// the source of the requested name. The returned definition is the concrete associated type
    /// member owned by the candidate trait, so the input and output [`DefId`]s may differ.
    fn trait_member_assoc(&self, trait_: TypeId, requested_assoc_type: DefId) -> Option<DefId> {
        let trait_def = self.types.nominal_def(trait_)?;
        if self.names[trait_def].kind != DefKind::Trait {
            return None;
        }
        let assoc_name = self.names[requested_assoc_type].name?;
        let ResolveResult::Found(member_assoc_type) =
            self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        if self.names[member_assoc_type].kind != DefKind::AssocType {
            return None;
        }
        Some(member_assoc_type)
    }

    fn logic_type_bindings(
        &self,
        projection_self: TypeId,
        impl_self: TypeId,
    ) -> Option<Vec<ImplSelfTypeArgBinding>> {
        let mut generic_vars = Vec::new();
        let mut concrete_terms = Vec::new();
        let projection_term = self.impl_self_type_term(
            projection_self,
            ImplSelfTermMode::Concrete,
            &mut generic_vars,
            &mut concrete_terms,
        )?;
        let impl_term = self.impl_self_type_term(
            impl_self,
            ImplSelfTermMode::ImplPattern,
            &mut generic_vars,
            &mut concrete_terms,
        )?;

        let mut db = Database::default();
        db.insert_clause(Clause {
            head: self.impl_self_unifies_term(projection_term.clone(), projection_term.clone()),
            body: None,
        });
        let query =
            Expr::Term(self.impl_self_unifies_term(projection_term.clone(), impl_term.clone()));

        let mut matched = false;
        let mut bindings = Vec::new();
        let mut query = db.query(query);
        while let Some(result) = query.prove_next() {
            let mut result_bindings = Vec::new();
            let mut result_type_bindings = Vec::new();
            for assignment in result {
                let var = assignment.get_lhs_variable();
                let Some(generic) = generic_vars
                    .iter()
                    .find_map(|(candidate, generic)| (candidate == var).then_some(*generic))
                else {
                    continue;
                };
                let rhs = assignment.rhs();
                let arg = Self::type_id_for_logic_term(&rhs, &concrete_terms)?;
                result_bindings.push((*var, rhs));
                let binding = ImplSelfTypeArgBinding {
                    projection_self,
                    impl_self,
                    generic,
                    arg,
                };
                result_type_bindings.push(binding);
            }
            let logic_bindings = result_bindings
                .iter()
                .map(|(var, rhs)| (*var, rhs.clone()))
                .collect::<Vec<_>>();
            let substituted_impl_term = Self::substitute_logic_vars(&impl_term, &logic_bindings);
            if substituted_impl_term != projection_term {
                continue;
            }
            matched = true;
            for binding in result_type_bindings {
                if !bindings.contains(&binding) {
                    bindings.push(binding);
                }
            }
        }

        matched.then_some(bindings)
    }

    fn type_id_for_logic_term(
        term: &LogicTerm<'cx>,
        concrete_terms: &[(LogicTerm<'cx>, TypeId)],
    ) -> Option<TypeId> {
        term::type_id_from_term(term).or_else(|| {
            concrete_terms
                .iter()
                .find_map(|(candidate, ty)| (candidate == term).then_some(*ty))
        })
    }

    fn substitute_logic_vars(
        term: &LogicTerm<'cx>,
        bindings: &[(term::LogicAtom<'cx>, LogicTerm<'cx>)],
    ) -> LogicTerm<'cx> {
        if term.args.is_empty() && term.functor.as_ref().starts_with('$') {
            if let Some((_, value)) = bindings
                .iter()
                .find(|(variable, _)| *variable == term.functor)
            {
                return value.clone();
            }
        }

        Term {
            functor: term.functor,
            args: term
                .args
                .iter()
                .map(|arg| Self::substitute_logic_vars(arg, bindings))
                .collect(),
        }
    }

    fn impl_self_type_term(
        &self,
        ty: TypeId,
        mode: ImplSelfTermMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        if matches!(mode, ImplSelfTermMode::ImplPattern) {
            if let Some(def) = Self::generic_def(self.types, ty) {
                return Some(Self::generic_var(self.ccx, def, ty, generic_vars));
            }
        }

        let term = match &self.types[ty] {
            Type::Array { elem, len } => Some(self.logic_term(
                func::ARRAY,
                vec![
                    self.impl_self_type_term(*elem, mode, generic_vars, concrete_terms)?,
                    self.array_len_term(*len),
                ],
            )),
            Type::Infer => Some(self.logic_term(func::INFER, vec![self.type_id_term(ty)])),
            Type::Primitive(primitive) => Some(self.primitive_term(*primitive)),
            Type::Path(path) => self.impl_self_path_term(path, mode, generic_vars, concrete_terms),
            Type::Reference { elem, is_mut } => {
                let elem = self.impl_self_type_term(*elem, mode, generic_vars, concrete_terms)?;
                if *is_mut {
                    Some(self.logic_term(func::REF, vec![self.logic_term(func::MUT, vec![elem])]))
                } else {
                    Some(self.logic_term(func::REF, vec![elem]))
                }
            }
            Type::Slice { elem } => Some(self.logic_term(
                func::SLICE,
                vec![self.impl_self_type_term(*elem, mode, generic_vars, concrete_terms)?],
            )),
            Type::Tuple { elems } => {
                let elems = elems
                    .iter()
                    .map(|elem| self.impl_self_type_term(*elem, mode, generic_vars, concrete_terms))
                    .collect::<Option<Vec<_>>>()?;
                Some(self.logic_term(func::TUPLE, elems))
            }
        }?;

        if matches!(mode, ImplSelfTermMode::Concrete)
            && concrete_terms
                .iter()
                .all(|(candidate, _)| candidate != &term)
        {
            concrete_terms.push((term.clone(), ty));
        }
        Some(term)
    }

    fn impl_self_path_term(
        &self,
        path: &PathType<'cx>,
        mode: ImplSelfTermMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        let def = match &path.resolution {
            PathTypeResolution::GenericParam(def) if matches!(mode, ImplSelfTermMode::Concrete) => {
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
            .map(|arg| self.impl_self_generic_arg_term(arg, mode, generic_vars, concrete_terms))
            .collect::<Option<Vec<_>>>()?;
        Some(self.logic_term(
            func::PATH,
            vec![
                self.logic_term(func::DEF, vec![self.def_id_term(def)]),
                self.logic_term(func::ARG, args),
            ],
        ))
    }

    fn impl_self_generic_arg_term(
        &self,
        arg: &GenericArg<'cx>,
        mode: ImplSelfTermMode,
        generic_vars: &mut Vec<(term::LogicAtom<'cx>, TypeId)>,
        concrete_terms: &mut Vec<(LogicTerm<'cx>, TypeId)>,
    ) -> Option<LogicTerm<'cx>> {
        match arg {
            GenericArg::Type(ty) => {
                self.impl_self_type_term(*ty, mode, generic_vars, concrete_terms)
            }
            GenericArg::Const(arg) => self.impl_self_const_arg_term(arg),
            GenericArg::AssocType { name, ty } => Some(self.logic_term(
                func::ASSOC_TYPE_ARG,
                vec![
                    self.name_term(name.as_ref()),
                    self.impl_self_type_term(*ty, mode, generic_vars, concrete_terms)?,
                ],
            )),
            GenericArg::AssocConst { name, value } => Some(self.logic_term(
                func::ASSOC_CONST_ARG,
                vec![
                    self.name_term(name.as_ref()),
                    self.impl_self_const_arg_term(value)?,
                ],
            )),
            GenericArg::Constraint { .. } | GenericArg::Unsupported => None,
        }
    }

    fn impl_self_const_arg_term(&self, arg: &ConstArg<'cx>) -> Option<LogicTerm<'cx>> {
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

    fn impl_self_unifies_term(
        &self,
        projection_self: LogicTerm<'cx>,
        impl_self: LogicTerm<'cx>,
    ) -> LogicTerm<'cx> {
        self.logic_term(pred::IMPL_SELF_UNIFIES, vec![projection_self, impl_self])
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

    fn logic_term(&self, functor: &str, args: Vec<LogicTerm<'cx>>) -> LogicTerm<'cx> {
        Term {
            functor: self.ccx.intern(functor),
            args,
        }
    }

    fn substitute_type(
        types: &mut InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<(TypeId, Vec<ImplSelfTypeArgBinding>)> {
        if let Some(binding) = Self::binding_for_generic(types, ty, bindings) {
            return Some((binding.arg, vec![binding]));
        }

        match types[ty].clone() {
            Type::Array { elem, len } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Array { elem, len }), used))
            }
            Type::Infer | Type::Primitive(_) => None,
            Type::Path(path) => {
                let (path, used) = Self::substitute_path_type(types, path, bindings)?;
                Some((types.intern_type(Type::Path(path)), used))
            }
            Type::Reference { elem, is_mut } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Reference { elem, is_mut }), used))
            }
            Type::Slice { elem } => {
                let (elem, used) = Self::substitute_type(types, elem, bindings)?;
                Some((types.intern_type(Type::Slice { elem }), used))
            }
            Type::Tuple { elems } => {
                let (elems, used) = Self::substitute_type_ids(types, elems, bindings);
                if used.is_empty() {
                    return None;
                }
                Some((types.intern_type(Type::Tuple { elems }), used))
            }
        }
    }

    fn substitute_path_type(
        types: &InferTypes<'cx>,
        path: PathType<'cx>,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<(PathType<'cx>, Vec<ImplSelfTypeArgBinding>)> {
        let mut used = Vec::new();
        let qself = path.qself.map(|qself| {
            let (self_, self_used) = Self::substitute_type_id(types, qself.self_, bindings);
            used.extend(self_used);
            let trait_ = qself.trait_.map(|trait_| {
                let (trait_, trait_used) = Self::substitute_type_id(types, trait_, bindings);
                used.extend(trait_used);
                trait_
            });
            crate::QSelf { self_, trait_ }
        });
        let segments = path
            .path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| Self::substitute_generic_argument(types, arg, bindings, &mut used))
                    .collect();
                crate::PathSegment {
                    name: segment.name,
                    args,
                }
            })
            .collect();

        let used = Self::unique_bindings(used);
        if used.is_empty() {
            return None;
        }
        Some((
            PathType {
                qself,
                path: crate::Path { segments },
                resolution: path.resolution,
            },
            used,
        ))
    }

    fn substitute_generic_argument(
        types: &InferTypes<'cx>,
        arg: GenericArg<'cx>,
        bindings: &[ImplSelfTypeArgBinding],
        used: &mut Vec<ImplSelfTypeArgBinding>,
    ) -> GenericArg<'cx> {
        match arg {
            GenericArg::Type(ty) => {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                GenericArg::Type(ty_id)
            }
            GenericArg::AssocType { name, ty } => {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                GenericArg::AssocType { name, ty: ty_id }
            }
            GenericArg::Const(arg) => GenericArg::Const(arg),
            GenericArg::AssocConst { name, value } => GenericArg::AssocConst { name, value },
            GenericArg::Constraint { name, bounds } => {
                let (bounds, bounds_used) = Self::substitute_type_bounds(types, bounds, bindings);
                used.extend(bounds_used);
                GenericArg::Constraint { name, bounds }
            }
            GenericArg::Unsupported => GenericArg::Unsupported,
        }
    }

    fn substitute_type_bounds(
        types: &InferTypes<'cx>,
        bounds: Vec<crate::TypeParamBound<'cx>>,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (Vec<crate::TypeParamBound<'cx>>, Vec<ImplSelfTypeArgBinding>) {
        let mut used = Vec::new();
        let bounds = bounds
            .into_iter()
            .map(|bound| {
                let (bound, bound_used) = Self::substitute_type_param_bound(types, bound, bindings);
                used.extend(bound_used);
                bound
            })
            .collect();
        (bounds, Self::unique_bindings(used))
    }

    fn substitute_type_param_bound(
        types: &InferTypes<'cx>,
        bound: crate::TypeParamBound<'cx>,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (crate::TypeParamBound<'cx>, Vec<ImplSelfTypeArgBinding>) {
        match bound {
            crate::TypeParamBound::Trait(path) => {
                let (path, used) = Self::substitute_path(types, path, bindings);
                (crate::TypeParamBound::Trait(path), used)
            }
            crate::TypeParamBound::Unsupported => (crate::TypeParamBound::Unsupported, Vec::new()),
        }
    }

    fn substitute_path(
        types: &InferTypes<'cx>,
        path: crate::Path<'cx>,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (crate::Path<'cx>, Vec<ImplSelfTypeArgBinding>) {
        let mut used = Vec::new();
        let segments = path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| {
                        let mut arg_used = Vec::new();
                        let arg =
                            Self::substitute_generic_argument(types, arg, bindings, &mut arg_used);
                        used.extend(arg_used);
                        arg
                    })
                    .collect();
                crate::PathSegment {
                    name: segment.name,
                    args,
                }
            })
            .collect();
        (crate::Path { segments }, Self::unique_bindings(used))
    }

    fn substitute_type_ids(
        types: &InferTypes<'cx>,
        tys: Vec<TypeId>,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (Vec<TypeId>, Vec<ImplSelfTypeArgBinding>) {
        let mut used = Vec::new();
        let tys = tys
            .into_iter()
            .map(|ty| {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                ty_id
            })
            .collect();
        (tys, Self::unique_bindings(used))
    }

    fn substitute_type_id(
        types: &InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> (TypeId, Vec<ImplSelfTypeArgBinding>) {
        if let Some(binding) = Self::binding_for_generic(types, ty, bindings) {
            return (binding.arg, vec![binding]);
        }
        (ty, Vec::new())
    }

    fn binding_for_generic(
        types: &InferTypes<'cx>,
        ty: TypeId,
        bindings: &[ImplSelfTypeArgBinding],
    ) -> Option<ImplSelfTypeArgBinding> {
        let generic_def = Self::generic_def(types, ty)?;
        bindings
            .iter()
            .copied()
            .find(|binding| Self::generic_def(types, binding.generic) == Some(generic_def))
    }

    fn unique_bindings(bindings: Vec<ImplSelfTypeArgBinding>) -> Vec<ImplSelfTypeArgBinding> {
        let mut unique = Vec::new();
        for binding in bindings {
            if !unique.contains(&binding) {
                unique.push(binding);
            }
        }
        unique
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

/// Performs projection logic operations:
///
/// * Loads trait-candidate or normalization rules
/// * Loads projection and trait facts needed by the selected rule set
/// * Loads Rust-side matching and substitution facts
/// * Queries trait-candidate and normalization predicates
struct ProjectionLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    projections: &'a ProjectionDb,
    types: &'a InferTypes<'cx>,
    trait_bound_facts: &'a [TraitBoundFact],
    assoc_type_impl_facts: &'a [AssocTypeImplFact],
    db: Database<term::LogicAtom<'cx>>,
}

impl<'a, 'cx> ProjectionLogic<'a, 'cx> {
    fn new(
        ccx: &'cx CommonCx,
        projections: &'a ProjectionDb,
        types: &'a InferTypes<'cx>,
        trait_bound_facts: &'a [TraitBoundFact],
        assoc_type_impl_facts: &'a [AssocTypeImplFact],
    ) -> Self {
        Self {
            ccx,
            projections,
            types,
            trait_bound_facts,
            assoc_type_impl_facts,
            db: Database::default(),
        }
    }

    fn load_projection_candidates(&mut self) {
        self.insert_same_type_rules();
        self.insert_candidate_rules();
        self.insert_projection_obligations();
        self.insert_trait_bounds();
        self.insert_type_equalities();
    }

    fn load_projection_normalizations(&mut self) {
        self.insert_same_type_rules();
        self.insert_normalization_rules();
        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_impl_self_matches();
        self.insert_type_binding_facts();
        self.insert_type_substitutions();
        self.insert_type_equalities();
    }

    fn load_impl_self_match_candidates(&mut self) {
        self.insert_same_type_rules();
        self.insert_impl_self_match_candidate_rules();
        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_type_equalities();
    }

    fn insert_same_type_rules(&mut self) {
        for clause in term::same_type_rules(self.ccx, term::PROJECTION_SAME_TYPE_RULES) {
            self.insert_clause(clause);
        }
    }

    fn insert_candidate_rules(&mut self) {
        for clause in term::projection_candidate_rules(self.ccx) {
            self.insert_clause(clause);
        }
    }

    fn insert_normalization_rules(&mut self) {
        for clause in term::projection_normalization_rules(self.ccx) {
            self.insert_clause(clause);
        }
    }

    fn insert_impl_self_match_candidate_rules(&mut self) {
        for clause in term::impl_self_match_candidate_rules(self.ccx) {
            self.insert_clause(clause);
        }
    }

    fn insert_projection_obligations(&mut self) {
        for obligation in self
            .projections
            .obligations
            .iter()
            .filter(|obligation| obligation.self_.is_some())
        {
            self.insert_clause(term::projection_obligation_clause(self.ccx, *obligation));
        }
    }

    fn insert_trait_bounds(&mut self) {
        for bound in self.trait_bound_facts {
            self.insert_clause(term::trait_bound_clause(self.ccx, *bound));
        }
    }

    fn insert_type_equalities(&mut self) {
        for left_index in 0..self.types.len() {
            let left = TypeId::new(left_index);
            for right in (left_index + 1)..self.types.len() {
                let right = TypeId::new(right);
                if self.types[left] != self.types[right] {
                    continue;
                }
                self.insert_clause(term::projection_type_equal_clause(self.ccx, left, right));
            }
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.projections.matches {
            self.insert_clause(term::projection_match_clause(self.ccx, *projection_match));
        }
    }

    fn insert_impl_assoc_types(&mut self) {
        for fact in self.assoc_type_impl_facts {
            self.insert_clause(term::impl_assoc_type_clause(self.ccx, *fact));
        }
    }

    fn insert_impl_self_matches(&mut self) {
        for match_ in &self.projections.impl_self_matches {
            self.insert_clause(term::impl_self_match_clause(self.ccx, *match_));
        }
    }

    fn insert_type_binding_facts(&mut self) {
        for binding in &self.projections.type_bindings {
            self.insert_clause(term::type_binding_clause(self.ccx, *binding));
        }
    }

    fn insert_type_substitutions(&mut self) {
        for substitution in &self.projections.type_substitutions {
            self.insert_clause(term::type_substitution_clause(self.ccx, *substitution));
        }
    }

    fn proves_candidate(
        &mut self,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
        trait_: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_candidate_query(
                self.ccx, projection, self_, assoc, trait_,
            ))
            .is_true()
    }

    fn proves_normalization(
        &mut self,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
        trait_: TypeId,
        value_ty: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_normalization_query(
                self.ccx, projection, self_, assoc, trait_, value_ty,
            ))
            .is_true()
    }

    fn impl_self_match_candidates(&mut self) -> Vec<ImplSelfMatch> {
        let mut candidates = Vec::new();
        let mut query = self
            .db
            .query(term::impl_self_match_candidate_query(self.ccx));
        while let Some(result) = query.prove_next() {
            let mut projection_self = None;
            let mut impl_self = None;
            for assignment in result {
                let variable = assignment.get_lhs_variable().as_ref();
                if variable == var::SELF {
                    projection_self = term::type_id_from_term(&assignment.rhs());
                } else if variable == var::IMPL_SELF {
                    impl_self = term::type_id_from_term(&assignment.rhs());
                }
            }
            let (Some(projection_self), Some(impl_self)) = (projection_self, impl_self) else {
                continue;
            };
            let candidate = ImplSelfMatch {
                projection_self,
                impl_self,
            };
            if !candidates.contains(&candidate) {
                candidates.push(candidate);
            }
        }
        candidates
    }

    fn insert_clause(&mut self, clause: term::LogicClause<'cx>) {
        self.db.insert_clause(clause);
    }
}
