//! Logic-backed associated type projection derivation.

use crate::{
    logic::term, AssocTypeImplFact, GenericArg, ImplSelfMatch, InferTypes, PathType,
    PathTypeResolution, ProjectionCandidate, ProjectionDb, ProjectionMatch,
    ProjectionNormalization, TraitBoundFact, Type, TypeBindingFact, TypeId, TypeSubstitution,
};
use logic_eval::Database;
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

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
        let candidates = self.derive_projection_candidates();
        self.projections.candidates.extend(candidates);

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

    fn derive_projection_candidates(&self) -> Vec<ProjectionCandidate> {
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_projection_candidates();

        let mut candidates = Vec::new();
        for obligation in &self.projections.obligations {
            let Some(self_ty_id) = obligation.self_ty_id else {
                continue;
            };
            if let Some(trait_ty_id) = obligation.trait_ty_id {
                if logic.proves_candidate(
                    obligation.projection_ty_id,
                    self_ty_id,
                    obligation.assoc_type,
                    trait_ty_id,
                ) {
                    candidates.push(ProjectionCandidate {
                        projection_ty_id: obligation.projection_ty_id,
                        self_ty_id,
                        assoc_type: obligation.assoc_type,
                        trait_ty_id,
                    });
                }
                continue;
            }
            for bound in self.trait_bound_facts {
                if logic.proves_candidate(
                    obligation.projection_ty_id,
                    self_ty_id,
                    obligation.assoc_type,
                    bound.trait_ty_id,
                ) {
                    candidates.push(ProjectionCandidate {
                        projection_ty_id: obligation.projection_ty_id,
                        self_ty_id,
                        assoc_type: obligation.assoc_type,
                        trait_ty_id: bound.trait_ty_id,
                    });
                }
            }
        }
        candidates
    }

    fn derive_projection_matches(&self) -> Vec<ProjectionMatch> {
        let trait_members = self.trait_members();
        let mut logic = ProjectionLogic::new(
            self.ccx,
            self.projections,
            self.types,
            self.trait_bound_facts,
            self.assoc_type_impl_facts,
        );
        logic.load_projection_matches(&trait_members);

        let mut matches = Vec::new();
        for candidate in &self.projections.candidates {
            for member in trait_members
                .iter()
                .filter(|member| member.matches_candidate(*candidate))
            {
                if logic.proves_match(
                    candidate.projection_ty_id,
                    candidate.self_ty_id,
                    member.member_assoc_type,
                    candidate.trait_ty_id,
                ) {
                    matches.push(ProjectionMatch {
                        projection_ty_id: candidate.projection_ty_id,
                        self_ty_id: candidate.self_ty_id,
                        assoc_type: member.member_assoc_type,
                        trait_ty_id: candidate.trait_ty_id,
                    });
                }
            }
        }
        matches
    }

    fn derive_impl_self_matches(&self) -> (Vec<ImplSelfMatch>, Vec<TypeBindingFact>) {
        let mut impl_self_matches = Vec::new();
        let mut type_bindings = Vec::new();
        for projection_match in &self.projections.matches {
            for impl_fact in self.assoc_type_impl_facts {
                if projection_match.assoc_type != impl_fact.assoc_type
                    || !self.same_type(projection_match.trait_ty_id, impl_fact.trait_ty_id)
                {
                    continue;
                }
                let Some(bindings) =
                    self.type_bindings(projection_match.self_ty_id, impl_fact.impl_self_ty_id)
                else {
                    continue;
                };
                let match_ = ImplSelfMatch {
                    projection_self_ty_id: projection_match.self_ty_id,
                    impl_self_ty_id: impl_fact.impl_self_ty_id,
                };
                if !impl_self_matches.contains(&match_) {
                    impl_self_matches.push(match_);
                }
                for binding in bindings {
                    if !type_bindings.contains(&binding) {
                        type_bindings.push(binding);
                    }
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
                .filter(|binding| binding.impl_self_ty_id == impl_fact.impl_self_ty_id)
            {
                let context = (binding.projection_self_ty_id, binding.impl_self_ty_id);
                if !contexts.contains(&context) {
                    contexts.push(context);
                }
            }

            for (projection_self_ty_id, impl_self_ty_id) in contexts {
                let impl_bindings = self
                    .projections
                    .type_bindings
                    .iter()
                    .filter(|binding| {
                        binding.projection_self_ty_id == projection_self_ty_id
                            && binding.impl_self_ty_id == impl_self_ty_id
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let Some((substituted_ty_id, used_bindings)) =
                    Self::substitute_type(self.types, impl_fact.value_ty_id, &impl_bindings)
                else {
                    continue;
                };
                for binding in used_bindings {
                    let substitution = TypeSubstitution {
                        projection_self_ty_id: binding.projection_self_ty_id,
                        impl_self_ty_id: binding.impl_self_ty_id,
                        value_ty_id: impl_fact.value_ty_id,
                        generic_ty_id: binding.generic_ty_id,
                        arg_ty_id: binding.arg_ty_id,
                        substituted_ty_id,
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
                        substitution.projection_self_ty_id == projection_match.self_ty_id
                            && substitution.impl_self_ty_id == impl_fact.impl_self_ty_id
                            && substitution.value_ty_id == impl_fact.value_ty_id
                    })
                    .map(|substitution| substitution.substituted_ty_id);
                for value_ty_id in std::iter::once(impl_fact.value_ty_id).chain(substituted_values)
                {
                    if logic.proves_normalization(
                        projection_match.projection_ty_id,
                        projection_match.self_ty_id,
                        projection_match.assoc_type,
                        projection_match.trait_ty_id,
                        value_ty_id,
                    ) {
                        let normalization = ProjectionNormalization {
                            projection_ty_id: projection_match.projection_ty_id,
                            self_ty_id: projection_match.self_ty_id,
                            assoc_type: projection_match.assoc_type,
                            trait_ty_id: projection_match.trait_ty_id,
                            value_ty_id,
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

    fn trait_members(&self) -> Vec<TraitMember> {
        self.projections
            .candidates
            .iter()
            .filter_map(|candidate| self.trait_member(*candidate))
            .collect()
    }

    fn trait_member(&self, candidate: ProjectionCandidate) -> Option<TraitMember> {
        let trait_def = self.nominal_def(candidate.trait_ty_id)?;
        if self.names[trait_def].kind != DefKind::Trait {
            return None;
        }
        let assoc_name = self.names[candidate.assoc_type].name?;
        let ResolveResult::Found(member_assoc_type) =
            self.names.member(trait_def, Namespace::Type, assoc_name)
        else {
            return None;
        };
        if self.names[member_assoc_type].kind != DefKind::AssocType {
            return None;
        }
        Some(TraitMember {
            trait_ty_id: candidate.trait_ty_id,
            requested_assoc_type: candidate.assoc_type,
            member_assoc_type,
        })
    }

    fn nominal_def(&self, ty_id: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.types[ty_id] else {
            return None;
        };
        let PathTypeResolution::Nominal(def) = path.resolution else {
            return None;
        };
        Some(def)
    }

    fn same_type(&self, left: TypeId, right: TypeId) -> bool {
        left == right || self.types[left] == self.types[right]
    }

    fn type_bindings(
        &self,
        projection_self_ty_id: TypeId,
        impl_self_ty_id: TypeId,
    ) -> Option<Vec<TypeBindingFact>> {
        let projection_path = self.path_type(projection_self_ty_id)?;
        let impl_path = self.path_type(impl_self_ty_id)?;
        if self.nominal_def(projection_self_ty_id)? != self.nominal_def(impl_self_ty_id)? {
            return None;
        }

        let [projection_segment] = projection_path.path.segments.as_slice() else {
            return None;
        };
        let [impl_segment] = impl_path.path.segments.as_slice() else {
            return None;
        };
        if projection_segment.args.len() != impl_segment.args.len() {
            return None;
        }

        let mut bindings = Vec::new();
        for (projection_arg, impl_arg) in projection_segment.args.iter().zip(&impl_segment.args) {
            let GenericArg::Type(arg_ty_id) = projection_arg else {
                return None;
            };
            let GenericArg::Type(generic_ty_id) = impl_arg else {
                return None;
            };
            Self::generic_def(self.types, *generic_ty_id)?;
            bindings.push(TypeBindingFact {
                projection_self_ty_id,
                impl_self_ty_id,
                generic_ty_id: *generic_ty_id,
                arg_ty_id: *arg_ty_id,
            });
        }

        Some(bindings)
    }

    fn substitute_type(
        types: &mut InferTypes<'cx>,
        ty_id: TypeId,
        bindings: &[TypeBindingFact],
    ) -> Option<(TypeId, Vec<TypeBindingFact>)> {
        if let Some(binding) = Self::binding_for_generic(types, ty_id, bindings) {
            return Some((binding.arg_ty_id, vec![binding]));
        }

        match types[ty_id].clone() {
            Type::Array { elem_id, len } => {
                let (elem_id, used) = Self::substitute_type(types, elem_id, bindings)?;
                Some((types.intern_type(Type::Array { elem_id, len }), used))
            }
            Type::Infer | Type::Primitive(_) => None,
            Type::Path(path) => {
                let (path, used) = Self::substitute_path_type(types, path, bindings)?;
                Some((types.intern_type(Type::Path(path)), used))
            }
            Type::Reference { elem_id, is_mut } => {
                let (elem_id, used) = Self::substitute_type(types, elem_id, bindings)?;
                Some((types.intern_type(Type::Reference { elem_id, is_mut }), used))
            }
            Type::Slice { elem_id } => {
                let (elem_id, used) = Self::substitute_type(types, elem_id, bindings)?;
                Some((types.intern_type(Type::Slice { elem_id }), used))
            }
            Type::Tuple { elem_ids } => {
                let (elem_ids, used) = Self::substitute_type_ids(types, elem_ids, bindings);
                if used.is_empty() {
                    return None;
                }
                Some((types.intern_type(Type::Tuple { elem_ids }), used))
            }
        }
    }

    fn substitute_path_type(
        types: &InferTypes<'cx>,
        path: PathType<'cx>,
        bindings: &[TypeBindingFact],
    ) -> Option<(PathType<'cx>, Vec<TypeBindingFact>)> {
        let mut used = Vec::new();
        let qself = path.qself.map(|qself| {
            let (self_ty_id, self_used) =
                Self::substitute_type_id(types, qself.self_ty_id, bindings);
            used.extend(self_used);
            let trait_ty_id = qself.trait_ty_id.map(|trait_ty_id| {
                let (trait_ty_id, trait_used) =
                    Self::substitute_type_id(types, trait_ty_id, bindings);
                used.extend(trait_used);
                trait_ty_id
            });
            crate::QSelf {
                self_ty_id,
                trait_ty_id,
            }
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
        bindings: &[TypeBindingFact],
        used: &mut Vec<TypeBindingFact>,
    ) -> GenericArg<'cx> {
        match arg {
            GenericArg::Type(ty) => {
                let (ty, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                GenericArg::Type(ty)
            }
            GenericArg::AssocType { name, ty_id } => {
                let (ty_id, ty_used) = Self::substitute_type_id(types, ty_id, bindings);
                used.extend(ty_used);
                GenericArg::AssocType { name, ty_id }
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
        bindings: &[TypeBindingFact],
    ) -> (Vec<crate::TypeParamBound<'cx>>, Vec<TypeBindingFact>) {
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
        bindings: &[TypeBindingFact],
    ) -> (crate::TypeParamBound<'cx>, Vec<TypeBindingFact>) {
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
        bindings: &[TypeBindingFact],
    ) -> (crate::Path<'cx>, Vec<TypeBindingFact>) {
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
        bindings: &[TypeBindingFact],
    ) -> (Vec<TypeId>, Vec<TypeBindingFact>) {
        let mut used = Vec::new();
        let tys = tys
            .into_iter()
            .map(|ty| {
                let (ty, ty_used) = Self::substitute_type_id(types, ty, bindings);
                used.extend(ty_used);
                ty
            })
            .collect();
        (tys, Self::unique_bindings(used))
    }

    fn substitute_type_id(
        types: &InferTypes<'cx>,
        ty_id: TypeId,
        bindings: &[TypeBindingFact],
    ) -> (TypeId, Vec<TypeBindingFact>) {
        if let Some(binding) = Self::binding_for_generic(types, ty_id, bindings) {
            return (binding.arg_ty_id, vec![binding]);
        }
        (ty_id, Vec::new())
    }

    fn binding_for_generic(
        types: &InferTypes<'cx>,
        ty_id: TypeId,
        bindings: &[TypeBindingFact],
    ) -> Option<TypeBindingFact> {
        let generic_def = Self::generic_def(types, ty_id)?;
        bindings
            .iter()
            .copied()
            .find(|binding| Self::generic_def(types, binding.generic_ty_id) == Some(generic_def))
    }

    fn unique_bindings(bindings: Vec<TypeBindingFact>) -> Vec<TypeBindingFact> {
        let mut unique = Vec::new();
        for binding in bindings {
            if !unique.contains(&binding) {
                unique.push(binding);
            }
        }
        unique
    }

    fn path_type(&self, ty_id: TypeId) -> Option<&PathType<'cx>> {
        let Type::Path(path) = &self.types[ty_id] else {
            return None;
        };
        Some(path)
    }

    fn generic_def(types: &InferTypes<'cx>, ty_id: TypeId) -> Option<DefId> {
        let Type::Path(path) = &types[ty_id] else {
            return None;
        };
        let PathTypeResolution::GenericParam(def) = path.resolution else {
            return None;
        };
        Some(def)
    }
}

#[derive(Clone, Copy)]
struct TraitMember {
    trait_ty_id: TypeId,
    requested_assoc_type: DefId,
    member_assoc_type: DefId,
}

impl TraitMember {
    fn matches_candidate(self, candidate: ProjectionCandidate) -> bool {
        self.trait_ty_id == candidate.trait_ty_id
            && self.requested_assoc_type == candidate.assoc_type
    }
}

/// Performs projection logic operations:
///
/// * Loads candidate, match, or normalization rules
/// * Loads projection and trait facts needed by the selected rule set
/// * Loads Rust-side matching and substitution facts
/// * Queries candidate, match, and normalization predicates
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

    fn load_projection_matches(&mut self, trait_members: &[TraitMember]) {
        self.insert_match_rules();
        self.insert_projection_candidates();
        self.insert_trait_members(trait_members);
        self.insert_impl_assoc_types();
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

    fn insert_match_rules(&mut self) {
        for clause in term::projection_match_rules(self.ccx) {
            self.insert_clause(clause);
        }
    }

    fn insert_normalization_rules(&mut self) {
        for clause in term::projection_normalization_rules(self.ccx) {
            self.insert_clause(clause);
        }
    }

    fn insert_projection_obligations(&mut self) {
        for obligation in &self.projections.obligations {
            let Some(self_ty_id) = obligation.self_ty_id else {
                continue;
            };
            self.insert_clause(term::projection_obligation_clause(
                self.ccx,
                *obligation,
                self_ty_id,
            ));
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

    fn insert_projection_candidates(&mut self) {
        for candidate in &self.projections.candidates {
            self.insert_clause(term::projection_candidate_clause(self.ccx, *candidate));
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.projections.matches {
            self.insert_clause(term::projection_match_clause(self.ccx, *projection_match));
        }
    }

    fn insert_trait_members(&mut self, trait_members: &[TraitMember]) {
        for member in trait_members {
            self.insert_clause(term::trait_member_clause(
                self.ccx,
                member.trait_ty_id,
                member.requested_assoc_type,
                member.member_assoc_type,
            ));
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
        projection_ty_id: TypeId,
        self_ty_id: TypeId,
        assoc_type: DefId,
        trait_ty_id: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_candidate_query(
                self.ccx,
                projection_ty_id,
                self_ty_id,
                assoc_type,
                trait_ty_id,
            ))
            .is_true()
    }

    fn proves_match(
        &mut self,
        projection_ty_id: TypeId,
        self_ty_id: TypeId,
        assoc_type: DefId,
        trait_ty_id: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_match_query(
                self.ccx,
                projection_ty_id,
                self_ty_id,
                assoc_type,
                trait_ty_id,
            ))
            .is_true()
    }

    fn proves_normalization(
        &mut self,
        projection_ty_id: TypeId,
        self_ty_id: TypeId,
        assoc_type: DefId,
        trait_ty_id: TypeId,
        value_ty_id: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_normalization_query(
                self.ccx,
                projection_ty_id,
                self_ty_id,
                assoc_type,
                trait_ty_id,
                value_ty_id,
            ))
            .is_true()
    }

    fn insert_clause(&mut self, clause: term::LogicClause<'cx>) {
        self.db.insert_clause(clause);
    }
}
