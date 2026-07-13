//! Logic database adapter for associated type projection normalization.

use super::type_shape_term::type_shape_clause;
use super::type_shape_term::TypeShapeMode;
use super::{
    term, ImplSelfGenericBinding, ImplSelfMatch, ProjectionDb, TypeShape, TypeShapeEncoder,
};
use crate::{
    logic::{
        self as logic_term, atom, def_id_from_term, symbol::Var, type_id_from_term, visit_left_var,
        Atom, InferLogic, LogicSessionToken, Term,
    },
    GenericArg, ImplAssocType, InferConstFacts, InferTypes, Path, PathType, PathTypeResolution,
    ProjectionNormalization, TraitBound, Type, TypeId, TypeParamBound,
};
use logic_eval::DatabaseCheckpoint;
use syn_sem_common::{CommonCx, Map, VecUniqueExt};
use syn_sem_name::DefId;

/// Performs projection logic operations:
///
/// * Loads trait-candidate or normalization rules
/// * Loads projection and trait clauses needed by the selected rule set
/// * Loads Rust-side matching and substitution clauses
/// * Queries trait-candidate and normalization predicates
pub(crate) struct ProjectionLogic<'cx> {
    initialized: bool,
    db_cursor: ProjectionDbCursor,
    preserve_generic_shapes: Map<TypeId, TypeShape<'cx>>,
    variable_generic_shapes: Map<TypeId, TypeShape<'cx>>,
}

impl<'cx> ProjectionLogic<'cx> {
    pub(crate) fn new(_: &LogicSessionToken) -> Self {
        Self {
            initialized: false,
            db_cursor: ProjectionDbCursor::default(),
            preserve_generic_shapes: Map::default(),
            variable_generic_shapes: Map::default(),
        }
    }

    pub(crate) fn initialize(
        &mut self,
        logic: &mut InferLogic<'cx>,
        ccx: &'cx CommonCx,
        types: &InferTypes<'cx>,
        trait_bounds: &[TraitBound],
        impl_assoc_types: &[ImplAssocType],
        const_facts: &InferConstFacts,
    ) {
        if self.initialized {
            return;
        }

        self.insert_common_rules(logic);
        self.insert_trait_bounds(logic, trait_bounds);
        self.insert_impl_assoc_types(logic, impl_assoc_types);
        self.insert_impl_assoc_values_without_bindings(logic, types, impl_assoc_types);
        self.insert_variable_generic_shapes(logic, ccx, types, impl_assoc_types, const_facts);
        logic.sync_type_classes(types);
        self.initialized = true;
    }

    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #explicit_projection_obligation($Projection, $Self, $Assoc, $Trait).
    /// #projection_candidate($Projection, $Self, $Assoc, $Trait) :-
    ///   #projection_obligation($Projection, $Self, $Assoc),
    ///   #trait_bound($Subject, $Trait), #same_type($Self, $Subject).
    /// #same_type($A, $A).
    /// #same_type($A, $B) :- #type_class($A, $Class), #type_class($B, $Class).
    ///
    /// #explicit_projection_obligation(projection, self, assoc, trait).
    /// #projection_obligation(projection, self, assoc).
    /// #trait_bound(subject, trait).
    /// #type_class(a, class).
    /// #type_class(b, class).
    pub(super) fn sync_projection_candidates(
        &mut self,
        logic: &mut InferLogic<'cx>,
        projections: &ProjectionDb,
        types: &InferTypes<'cx>,
    ) {
        self.sync_projection_obligations(logic, projections);
        logic.sync_type_classes(types);
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
    /// #same_type($A, $B) :- #type_class($A, $Class), #type_class($B, $Class).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #impl_self_match(self, impl_self).
    /// #impl_assoc_value_without_bindings(impl_self, value).
    /// #type_binding(self, impl_self, generic, arg).
    /// #type_substitution(self, impl_self, value, generic, arg, substituted).
    /// #type_class(a, class).
    /// #type_class(b, class).
    pub(super) fn sync_projection_normalizations(
        &mut self,
        logic: &mut InferLogic<'cx>,
        ccx: &'cx CommonCx,
        projections: &ProjectionDb,
        types: &InferTypes<'cx>,
        const_facts: &InferConstFacts,
    ) {
        self.sync_projection_matches(logic, ccx, projections, types, const_facts);
        self.sync_impl_self_matches(logic, projections);
        self.sync_type_bindings(logic, projections);
        self.sync_type_substitutions(logic, projections);
        logic.sync_type_classes(types);
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
    /// #same_type($A, $B) :- #type_class($A, $Class), #type_class($B, $Class).
    ///
    /// #projection_match(projection, self, assoc, trait).
    /// #impl_assoc_type(impl_self, impl_trait, assoc, value).
    /// #type_shape(self, #preserve_generics, shape).
    /// #type_shape(impl_self, #variable_generics, shape).
    /// #type_class(a, class).
    /// #type_class(b, class).
    pub(super) fn sync_impl_self_match_inputs(
        &mut self,
        logic: &mut InferLogic<'cx>,
        ccx: &'cx CommonCx,
        projections: &ProjectionDb,
        types: &InferTypes<'cx>,
        const_facts: &InferConstFacts,
    ) {
        self.sync_projection_matches(logic, ccx, projections, types, const_facts);
        logic.sync_type_classes(types);
    }

    fn insert_common_rules(&mut self, logic: &mut InferLogic<'cx>) {
        for clause in logic_term::same_type_rules() {
            logic.db.insert_clause(clause);
        }
    }

    pub(super) fn begin_projection_candidate_queries(
        &mut self,
        logic: &mut InferLogic<'cx>,
    ) -> DatabaseCheckpoint {
        let checkpoint = logic.db.checkpoint();
        for clause in term::projection_candidate_rules() {
            logic.db.insert_clause(clause);
        }
        checkpoint
    }

    pub(super) fn begin_projection_normalization_query(
        &mut self,
        logic: &mut InferLogic<'cx>,
    ) -> DatabaseCheckpoint {
        let checkpoint = logic.db.checkpoint();
        for clause in term::projection_normalization_rules() {
            logic.db.insert_clause(clause);
        }
        checkpoint
    }

    pub(super) fn begin_impl_self_match_query(
        &mut self,
        logic: &mut InferLogic<'cx>,
    ) -> DatabaseCheckpoint {
        let checkpoint = logic.db.checkpoint();
        for clause in term::impl_self_match_candidate_rules() {
            logic.db.insert_clause(clause);
        }
        for clause in term::impl_self_match_rules() {
            logic.db.insert_clause(clause);
        }
        checkpoint
    }

    pub(super) fn end_query(
        &mut self,
        logic: &mut InferLogic<'cx>,
        checkpoint: DatabaseCheckpoint,
    ) {
        logic.db.revert(checkpoint);
    }

    fn sync_projection_obligations(
        &mut self,
        logic: &mut InferLogic<'cx>,
        projections: &ProjectionDb,
    ) {
        for obligation in &projections.obligations[self.db_cursor.obligations..] {
            logic
                .db
                .insert_clause(term::projection_obligation_clause(*obligation));
        }
        self.db_cursor.obligations = projections.obligations.len();
    }

    fn insert_trait_bounds(&mut self, logic: &mut InferLogic<'cx>, trait_bounds: &[TraitBound]) {
        for bound in trait_bounds {
            logic.db.insert_clause(term::trait_bound_clause(*bound));
        }
    }

    fn sync_projection_matches(
        &mut self,
        logic: &mut InferLogic<'cx>,
        ccx: &'cx CommonCx,
        projections: &ProjectionDb,
        types: &InferTypes<'cx>,
        const_facts: &InferConstFacts,
    ) {
        let encoder = TypeShapeEncoder::new(ccx, types, const_facts);
        for projection_match in &projections.projection_matches[self.db_cursor.projection_matches..]
        {
            logic
                .db
                .insert_clause(term::projection_match_clause(*projection_match));
            let ty = projection_match.self_;
            if self.preserve_generic_shapes.contains_key(&ty) {
                continue;
            }
            let Some(shape) = encoder.encode(ty, TypeShapeMode::PreserveGenerics) else {
                continue;
            };
            let shape_term = shape.term.clone();
            self.preserve_generic_shapes.insert(ty, shape);
            logic.db.insert_clause(type_shape_clause(
                ty,
                TypeShapeMode::PreserveGenerics,
                shape_term,
            ));
        }
        self.db_cursor.projection_matches = projections.projection_matches.len();
    }

    fn insert_impl_assoc_types(
        &mut self,
        logic: &mut InferLogic<'cx>,
        impl_assoc_types: &[ImplAssocType],
    ) {
        for impl_assoc_type in impl_assoc_types {
            logic
                .db
                .insert_clause(term::impl_assoc_type_clause(*impl_assoc_type));
        }
    }

    fn insert_impl_assoc_values_without_bindings(
        &mut self,
        logic: &mut InferLogic<'cx>,
        types: &InferTypes<'cx>,
        impl_assoc_types: &[ImplAssocType],
    ) {
        for impl_assoc_type in impl_assoc_types {
            if Self::type_contains_generic_param(types, impl_assoc_type.value_ty) {
                continue;
            }
            logic
                .db
                .insert_clause(term::impl_assoc_value_without_bindings_clause(
                    impl_assoc_type.impl_self,
                    impl_assoc_type.value_ty,
                ));
        }
    }

    fn sync_impl_self_matches(&mut self, logic: &mut InferLogic<'cx>, projections: &ProjectionDb) {
        for match_ in &projections.impl_self_matches[self.db_cursor.impl_self_matches..] {
            logic
                .db
                .insert_clause(term::impl_self_match_clause(*match_));
        }
        self.db_cursor.impl_self_matches = projections.impl_self_matches.len();
    }

    fn sync_type_bindings(&mut self, logic: &mut InferLogic<'cx>, projections: &ProjectionDb) {
        for binding in &projections.impl_self_generic_bindings[self.db_cursor.type_bindings..] {
            logic.db.insert_clause(term::type_binding_clause(*binding));
        }
        self.db_cursor.type_bindings = projections.impl_self_generic_bindings.len();
    }

    fn sync_type_substitutions(&mut self, logic: &mut InferLogic<'cx>, projections: &ProjectionDb) {
        for substitution in &projections.type_substitutions[self.db_cursor.type_substitutions..] {
            logic
                .db
                .insert_clause(term::type_substitution_clause(*substitution));
        }
        self.db_cursor.type_substitutions = projections.type_substitutions.len();
    }

    fn insert_variable_generic_shapes(
        &mut self,
        logic: &mut InferLogic<'cx>,
        ccx: &'cx CommonCx,
        types: &InferTypes<'cx>,
        impl_assoc_types: &[ImplAssocType],
        const_facts: &InferConstFacts,
    ) {
        let encoder = TypeShapeEncoder::new(ccx, types, const_facts);
        let mut impl_self_tys = Vec::new();
        for impl_assoc_type in impl_assoc_types {
            impl_self_tys.push_unique(impl_assoc_type.impl_self);
        }
        for impl_self in impl_self_tys {
            let Some(shape) = encoder.encode(impl_self, TypeShapeMode::VariableGenerics) else {
                continue;
            };
            let shape_term = shape.term.clone();
            self.variable_generic_shapes.insert(impl_self, shape);
            logic.db.insert_clause(type_shape_clause(
                impl_self,
                TypeShapeMode::VariableGenerics,
                shape_term,
            ));
        }
    }

    pub(super) fn candidate_traits(
        &mut self,
        logic: &mut InferLogic<'cx>,
        projection: TypeId,
        self_: TypeId,
        assoc: DefId,
    ) -> Vec<TypeId> {
        let mut traits = Vec::new();
        let mut qcx = logic.db.query(term::projection_candidate_trait_query(
            projection, self_, assoc,
        ));
        while let Some(answer) = qcx.prove_next() {
            let Some(trait_) = answer
                .get(&Atom::from(Var::Trait))
                .and_then(|term| type_id_from_term(&term))
            else {
                continue;
            };
            traits.push_unique(trait_);
        }
        traits
    }

    pub(super) fn normalizations(
        &mut self,
        logic: &mut InferLogic<'cx>,
    ) -> Vec<ProjectionNormalization> {
        let mut normalizations = Vec::new();
        let mut qcx = logic.db.query(term::projection_normalization_query());
        while let Some(answer) = qcx.prove_next() {
            let projection = answer
                .get(&Atom::from(Var::Projection))
                .and_then(|term| type_id_from_term(&term));
            let self_ = answer
                .get(&Atom::from(Var::SelfTy))
                .and_then(|term| type_id_from_term(&term));
            let assoc = answer
                .get(&Atom::from(Var::Assoc))
                .and_then(|term| def_id_from_term(&term));
            let trait_ = answer
                .get(&Atom::from(Var::Trait))
                .and_then(|term| type_id_from_term(&term));
            let Some(value_ty) = answer
                .get(&Atom::from(Var::Value))
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
        logic: &mut InferLogic<'cx>,
    ) -> (Vec<ImplSelfMatch>, Vec<ImplSelfGenericBinding>) {
        let mut impl_self_matches = Vec::new();
        let mut generic_bindings = Vec::new();
        let mut qcx = logic.db.query(term::impl_self_match_query());
        while let Some(answer) = qcx.prove_next() {
            let projection_self = answer
                .get(&Atom::from(Var::SelfTy))
                .and_then(|term| type_id_from_term(&term));
            let impl_self = answer
                .get(&Atom::from(Var::ImplSelf))
                .and_then(|term| type_id_from_term(&term));
            let projection_self_shape = answer.get(&Atom::from(Var::Shape));
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
        projection_self_shape: &Term<'cx>,
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

    fn type_contains_generic_param(types: &InferTypes<'cx>, ty: TypeId) -> bool {
        match &types[ty] {
            Type::Array { elem, .. } | Type::Reference { elem, .. } | Type::Slice { elem } => {
                Self::type_contains_generic_param(types, *elem)
            }
            Type::Infer | Type::Primitive(_) => false,
            Type::Path(path) => Self::path_contains_generic_param(types, path),
            Type::Tuple { elems } => elems
                .iter()
                .any(|elem| Self::type_contains_generic_param(types, *elem)),
        }
    }

    fn path_contains_generic_param(types: &InferTypes<'cx>, path: &PathType<'cx>) -> bool {
        if let Some(qself) = path.qself {
            let trait_contains_generic = match qself.trait_ {
                Some(trait_) => Self::type_contains_generic_param(types, trait_),
                None => false,
            };
            if Self::type_contains_generic_param(types, qself.self_) || trait_contains_generic {
                return true;
            }
        }
        matches!(path.resolution, PathTypeResolution::GenericParam(_))
            || Self::path_args_contain_generic_param(types, &path.path)
    }

    fn path_args_contain_generic_param(types: &InferTypes<'cx>, path: &Path<'cx>) -> bool {
        path.segments.iter().any(|segment| {
            segment
                .args
                .iter()
                .any(|arg| Self::generic_arg_contains_generic_param(types, arg))
        })
    }

    fn generic_arg_contains_generic_param(types: &InferTypes<'cx>, arg: &GenericArg<'cx>) -> bool {
        match arg {
            GenericArg::Type(ty) => Self::type_contains_generic_param(types, *ty),
            GenericArg::AssocType { ty, .. } => Self::type_contains_generic_param(types, *ty),
            GenericArg::Constraint { bounds, .. } => bounds
                .iter()
                .any(|bound| Self::type_param_bound_contains_generic_param(types, bound)),
            GenericArg::Const(_) | GenericArg::AssocConst { .. } | GenericArg::Unsupported => false,
        }
    }

    fn type_param_bound_contains_generic_param(
        types: &InferTypes<'cx>,
        bound: &TypeParamBound<'cx>,
    ) -> bool {
        match bound {
            TypeParamBound::Trait(path) => Self::path_args_contain_generic_param(types, path),
            TypeParamBound::Unsupported => false,
        }
    }

    fn type_id_for_logic_term(
        term: &Term<'cx>,
        term_types: &Map<Term<'cx>, TypeId>,
    ) -> Option<TypeId> {
        type_id_from_term(term).or_else(|| term_types.get(term).copied())
    }
}

/// Tracks the prefixes of append-only projection facts synchronized into logic.
#[derive(Default)]
struct ProjectionDbCursor {
    obligations: usize,
    projection_matches: usize,
    impl_self_matches: usize,
    type_bindings: usize,
    type_substitutions: usize,
}

#[cfg(test)]
mod tests {
    use crate::LogicSession;

    #[test]
    fn phase_rules_are_reverted_without_removing_common_rules() {
        LogicSession::default().with_projection(|logic, projection| {
            projection.insert_common_rules(logic);
            let common_clause_count = logic.db.clauses().count();

            let checkpoint = projection.begin_impl_self_match_query(logic);
            assert!(logic.db.clauses().count() > common_clause_count);

            projection.end_query(logic, checkpoint);
            assert_eq!(logic.db.clauses().count(), common_clause_count);
        });
    }
}
