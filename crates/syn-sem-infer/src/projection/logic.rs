//! Logic database adapter for associated type projection normalization.

use super::type_shape_term::type_shape_clause;
use super::type_shape_term::TypeShapeMode;
use super::{
    term, ImplSelfGenericBinding, ImplSelfMatch, ProjectionDb, TypeShape, TypeShapeEncoder,
};
use crate::{
    logic::{
        self as logic_term, atom, def_id_from_term, symbol::var, type_id_from_term, visit_left_var,
        LogicAtom, LogicTerm,
    },
    GenericArg, ImplAssocType, InferConstFacts, InferTypes, Path, PathType, PathTypeResolution,
    ProjectionNormalization, TraitBound, Type, TypeId, TypeParamBound,
};
use logic_eval::Database;
use syn_sem_common::{CommonCx, Map, VecUniqueExt};
use syn_sem_name::DefId;

/// Performs projection logic operations:
///
/// * Loads trait-candidate or normalization rules
/// * Loads projection and trait clauses needed by the selected rule set
/// * Loads Rust-side matching and substitution clauses
/// * Queries trait-candidate and normalization predicates
pub(super) struct ProjectionLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    projections: &'a ProjectionDb,
    types: &'a InferTypes<'cx>,
    trait_bounds: &'a [TraitBound],
    impl_assoc_types: &'a [ImplAssocType],
    const_facts: &'a InferConstFacts,
    preserve_generic_shapes: Map<TypeId, TypeShape<'cx>>,
    variable_generic_shapes: Map<TypeId, TypeShape<'cx>>,
    db: Database<LogicAtom<'cx>>,
}

impl<'a, 'cx> ProjectionLogic<'a, 'cx> {
    pub(super) fn new(
        ccx: &'cx CommonCx,
        projections: &'a ProjectionDb,
        types: &'a InferTypes<'cx>,
        trait_bounds: &'a [TraitBound],
        impl_assoc_types: &'a [ImplAssocType],
        const_facts: &'a InferConstFacts,
    ) -> Self {
        Self {
            ccx,
            projections,
            types,
            trait_bounds,
            impl_assoc_types,
            const_facts,
            preserve_generic_shapes: Map::default(),
            variable_generic_shapes: Map::default(),
            db: Database::default(),
        }
    }

    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).
    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #projection_obligation($Projection, $Self, $Assoc),
    ///   #trait_bound($Subject, $Trait), #same_type($Self, $Subject).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #explicit_projection_obligation(projection, self, assoc, trait).
    /// #projection_obligation(projection, self, assoc).
    /// #trait_bound(subject, trait).
    /// #type_equal(a, b).
    pub(super) fn load_projection_candidates(&mut self) {
        self.insert_candidate_rules();
        self.insert_same_type_rules();

        self.insert_projection_obligations();
        self.insert_trait_bounds();
        self.insert_type_equalities();
    }

    /// #projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Value) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Trait, $ImplTrait), #impl_self_match($Self, $ImplSelf),
    ///   #impl_assoc_value_without_bindings($ImplSelf, $Value).
    /// #projection_normalizes_to($Projection, $Self, $Assoc, $Trait, $Substituted) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Trait, $ImplTrait), #impl_self_match($Self, $ImplSelf),
    ///   #type_binding($Self, $ImplSelf, $Generic, $Arg),
    ///   #type_substitution($Self, $ImplSelf, $Value, $Generic, $Arg, $Substituted).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #impl_self_match(self, impl_self).
    /// #impl_assoc_value_without_bindings(impl_self, value).
    /// #type_binding(self, impl_self, generic, arg).
    /// #type_substitution(self, impl_self, value, generic, arg, substituted).
    /// #type_equal(a, b).
    pub(super) fn load_projection_normalizations(&mut self) {
        self.insert_normalization_rules();
        self.insert_same_type_rules();

        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_impl_assoc_values_without_bindings();
        self.insert_impl_self_matches();
        self.insert_type_bindings();
        self.insert_type_substitutions();
        self.insert_type_equalities();
    }

    /// #impl_self_match($Self, $ImplSelf) :-
    ///   #impl_self_match_candidate($Self, $ImplSelf),
    ///   #type_shape($Self, #preserve_generics, $Shape),
    ///   #type_shape($ImplSelf, #variable_generics, $Shape).
    /// #impl_self_match_candidate($Self, $ImplSelf) :-
    ///   #projection_match($Projection, $Self, $Assoc, $Trait),
    ///   #impl_assoc_type($ImplSelf, $ImplTrait, $Assoc, $Value),
    ///   #same_type($Trait, $ImplTrait).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_equal($A, $B).
    /// #same_type($A, $B) :- #type_equal($B, $A).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #type_shape(self, #preserve_generics, shape).
    /// #type_shape(impl_self, #variable_generics, shape).
    /// #type_equal(a, b).
    pub(super) fn load_impl_self_matches(&mut self) {
        self.insert_impl_self_match_rules();
        self.insert_impl_self_match_candidate_rules();
        self.insert_same_type_rules();

        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_type_shapes();
        self.insert_type_equalities();
    }

    fn insert_same_type_rules(&mut self) {
        for clause in logic_term::same_type_rules(self.ccx, term::PROJECTION_SAME_TYPE_RULES) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_candidate_rules(&mut self) {
        for clause in term::projection_candidate_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_normalization_rules(&mut self) {
        for clause in term::projection_normalization_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_impl_self_match_candidate_rules(&mut self) {
        for clause in term::impl_self_match_candidate_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_impl_self_match_rules(&mut self) {
        for clause in term::impl_self_match_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
    }

    fn insert_projection_obligations(&mut self) {
        for obligation in &self.projections.obligations {
            self.db
                .insert_clause(term::projection_obligation_clause(self.ccx, *obligation));
        }
    }

    fn insert_trait_bounds(&mut self) {
        for bound in self.trait_bounds {
            self.db
                .insert_clause(term::trait_bound_clause(self.ccx, *bound));
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
                self.db
                    .insert_clause(term::projection_type_equal_clause(self.ccx, left, right));
            }
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.projections.projection_matches {
            self.db
                .insert_clause(term::projection_match_clause(self.ccx, *projection_match));
        }
    }

    fn insert_impl_assoc_types(&mut self) {
        for impl_assoc_type in self.impl_assoc_types {
            self.db
                .insert_clause(term::impl_assoc_type_clause(self.ccx, *impl_assoc_type));
        }
    }

    fn insert_impl_assoc_values_without_bindings(&mut self) {
        for impl_assoc_type in self.impl_assoc_types {
            if self.type_contains_generic_param(impl_assoc_type.value_ty) {
                continue;
            }
            self.db
                .insert_clause(term::impl_assoc_value_without_bindings_clause(
                    self.ccx,
                    impl_assoc_type.impl_self,
                    impl_assoc_type.value_ty,
                ));
        }
    }

    fn insert_impl_self_matches(&mut self) {
        for match_ in &self.projections.impl_self_matches {
            self.db
                .insert_clause(term::impl_self_match_clause(self.ccx, *match_));
        }
    }

    fn insert_type_bindings(&mut self) {
        for binding in &self.projections.impl_self_generic_bindings {
            self.db
                .insert_clause(term::type_binding_clause(self.ccx, *binding));
        }
    }

    fn insert_type_substitutions(&mut self) {
        for substitution in &self.projections.type_substitutions {
            self.db
                .insert_clause(term::type_substitution_clause(self.ccx, *substitution));
        }
    }

    fn insert_type_shapes(&mut self) {
        let encoder = TypeShapeEncoder::new(self.ccx, self.types, self.const_facts);
        let mut preserve_generic_tys = Vec::new();
        for projection_match in &self.projections.projection_matches {
            preserve_generic_tys.push_unique(projection_match.self_);
        }
        for ty in preserve_generic_tys {
            let Some(shape) = encoder.encode(ty, TypeShapeMode::PreserveGenerics) else {
                continue;
            };
            let shape_term = shape.term.clone();
            self.preserve_generic_shapes.insert(ty, shape);
            self.db.insert_clause(type_shape_clause(
                self.ccx,
                ty,
                TypeShapeMode::PreserveGenerics,
                shape_term,
            ));
        }

        let mut impl_self_tys = Vec::new();
        for impl_assoc_type in self.impl_assoc_types {
            impl_self_tys.push_unique(impl_assoc_type.impl_self);
        }
        for impl_self in impl_self_tys {
            let Some(shape) = encoder.encode(impl_self, TypeShapeMode::VariableGenerics) else {
                continue;
            };
            let shape_term = shape.term.clone();
            self.variable_generic_shapes.insert(impl_self, shape);
            self.db.insert_clause(type_shape_clause(
                self.ccx,
                impl_self,
                TypeShapeMode::VariableGenerics,
                shape_term,
            ));
        }
    }

    pub(super) fn candidate_traits(
        &mut self,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
    ) -> Vec<TypeId> {
        let mut traits = Vec::new();
        let mut qcx = self.db.query(term::projection_candidate_trait_query(
            self.ccx, projection, self_, assoc,
        ));
        while let Some(answer) = qcx.prove_next() {
            let Some(trait_) = answer
                .get(var::TRAIT)
                .and_then(|term| type_id_from_term(&term))
            else {
                continue;
            };
            traits.push_unique(trait_);
        }
        traits
    }

    pub(super) fn normalizations(&mut self) -> Vec<ProjectionNormalization> {
        let mut normalizations = Vec::new();
        let mut qcx = self
            .db
            .query(term::projection_normalization_query(self.ccx));
        while let Some(answer) = qcx.prove_next() {
            let projection = answer
                .get(var::PROJECTION)
                .and_then(|term| type_id_from_term(&term));
            let self_ = answer
                .get(var::SELF)
                .and_then(|term| type_id_from_term(&term));
            let assoc = answer
                .get(var::ASSOC)
                .and_then(|term| def_id_from_term(&term));
            let trait_ = answer
                .get(var::TRAIT)
                .and_then(|term| type_id_from_term(&term));
            let Some(value_ty) = answer
                .get(var::VALUE)
                .and_then(|term| type_id_from_term(&term))
            else {
                continue;
            };
            let (Some(projection), Some(self_), Some(assoc), Some(trait_)) =
                (projection, self_, assoc, trait_)
            else {
                continue;
            };
            normalizations.push_unique(ProjectionNormalization {
                projection,
                self_,
                assoc,
                trait_,
                value_ty,
            });
        }
        normalizations
    }

    pub(super) fn impl_self_matches_and_generic_bindings(
        &mut self,
    ) -> (Vec<ImplSelfMatch>, Vec<ImplSelfGenericBinding>) {
        let mut impl_self_matches = Vec::new();
        let mut generic_bindings = Vec::new();
        let mut qcx = self.db.query(term::impl_self_match_query(self.ccx));
        while let Some(answer) = qcx.prove_next() {
            let projection_self = answer
                .get(var::SELF)
                .and_then(|term| type_id_from_term(&term));
            let impl_self = answer
                .get(var::IMPL_SELF)
                .and_then(|term| type_id_from_term(&term));
            let projection_self_shape = answer.get(var::SHAPE);
            let (Some(projection_self), Some(impl_self), Some(projection_self_shape)) =
                (projection_self, impl_self, projection_self_shape)
            else {
                continue;
            };

            let match_ = ImplSelfMatch {
                projection_self,
                impl_self,
            };
            impl_self_matches.push_unique(match_);
            for binding in self.impl_self_generic_bindings(match_, &projection_self_shape) {
                generic_bindings.push_unique(binding);
            }
        }
        (impl_self_matches, generic_bindings)
    }

    /// Materializes impl-self generic bindings from a successful self-type shape match.
    ///
    /// For `<Vec<u32> as Iterator>::Item` matched against `impl<T> Iterator for Vec<T>`, this finds
    /// that the impl generic `T` in the `Vec<T>` is bound to the projection argument `u32`.
    fn impl_self_generic_bindings(
        &self,
        match_: ImplSelfMatch,
        projection_self_shape: &LogicTerm<'cx>,
    ) -> Vec<ImplSelfGenericBinding> {
        let Some(preserve_generic_shape) =
            self.preserve_generic_shapes.get(&match_.projection_self)
        else {
            return Vec::new();
        };
        let Some(variable_generic_shape) = self.variable_generic_shapes.get(&match_.impl_self)
        else {
            return Vec::new();
        };

        let mut var_bindings = Vec::new();
        visit_left_var(
            &variable_generic_shape.term,
            projection_self_shape,
            &mut |var, rhs| {
                var_bindings.push((var, rhs));
            },
        );

        let mut bindings = Vec::new();
        for (var, rhs) in var_bindings {
            let var_term = atom(var);
            let Some(generic) =
                Self::type_id_for_logic_term(&var_term, &variable_generic_shape.term_types)
            else {
                continue;
            };
            let Some(arg) = Self::type_id_for_logic_term(rhs, &preserve_generic_shape.term_types)
            else {
                continue;
            };
            let binding = ImplSelfGenericBinding {
                projection_self: match_.projection_self,
                impl_self: match_.impl_self,
                generic,
                arg,
            };
            bindings.push_unique(binding);
        }
        bindings
    }

    fn type_contains_generic_param(&self, ty: TypeId) -> bool {
        match &self.types[ty] {
            Type::Array { elem, .. } | Type::Reference { elem, .. } | Type::Slice { elem } => {
                self.type_contains_generic_param(*elem)
            }
            Type::Infer | Type::Primitive(_) => false,
            Type::Path(path) => self.path_contains_generic_param(path),
            Type::Tuple { elems } => elems
                .iter()
                .any(|elem| self.type_contains_generic_param(*elem)),
        }
    }

    fn path_contains_generic_param(&self, path: &PathType<'cx>) -> bool {
        if let Some(qself) = path.qself {
            let trait_contains_generic = match qself.trait_ {
                Some(trait_) => self.type_contains_generic_param(trait_),
                None => false,
            };
            if self.type_contains_generic_param(qself.self_) || trait_contains_generic {
                return true;
            }
        }
        matches!(path.resolution, PathTypeResolution::GenericParam(_))
            || self.path_args_contain_generic_param(&path.path)
    }

    fn path_args_contain_generic_param(&self, path: &Path<'cx>) -> bool {
        path.segments.iter().any(|segment| {
            segment
                .args
                .iter()
                .any(|arg| self.generic_arg_contains_generic_param(arg))
        })
    }

    fn generic_arg_contains_generic_param(&self, arg: &GenericArg<'cx>) -> bool {
        match arg {
            GenericArg::Type(ty) => self.type_contains_generic_param(*ty),
            GenericArg::AssocType { ty, .. } => self.type_contains_generic_param(*ty),
            GenericArg::Constraint { bounds, .. } => bounds
                .iter()
                .any(|bound| self.type_param_bound_contains_generic_param(bound)),
            GenericArg::Const(_) | GenericArg::AssocConst { .. } | GenericArg::Unsupported => false,
        }
    }

    fn type_param_bound_contains_generic_param(&self, bound: &TypeParamBound<'cx>) -> bool {
        match bound {
            TypeParamBound::Trait(path) => self.path_args_contain_generic_param(path),
            TypeParamBound::Unsupported => false,
        }
    }

    fn type_id_for_logic_term(
        term: &LogicTerm<'cx>,
        term_types: &Map<LogicTerm<'cx>, TypeId>,
    ) -> Option<TypeId> {
        type_id_from_term(term).or_else(|| term_types.get(term).copied())
    }
}
