//! Logic-backed associated type projection derivation.

use crate::{
    GenericArgument, ImplSelfMatch, InferDb, PathType, PathTypeResolution, ProjectionCandidate,
    ProjectionMatch, ProjectionNormalization, Type, TypeBindingFact, TypeId, TypeSubstitution,
};
use logic_eval::Database;
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

use crate::logic::term;

pub(super) fn derive<'cx>(ccx: &'cx CommonCx, db: &mut InferDb<'cx>, names: &NameDb<'cx>) {
    let mut logic = LogicCx { ccx, db, names };
    logic.derive_projection_candidates();
    logic.derive_projection_matches();
    logic.derive_impl_self_matches();
    logic.derive_type_substitutions();
    logic.derive_projection_normalizations();
}

struct LogicCx<'a, 'cx> {
    ccx: &'cx CommonCx,
    db: &'a mut InferDb<'cx>,
    names: &'a NameDb<'cx>,
}

impl<'a, 'cx> LogicCx<'a, 'cx> {
    fn derive_projection_candidates(&mut self) {
        let mut logic = ProjectionLogic::new(self.ccx, self.db);
        logic.load_projection_candidates();
        let obligations = self.db.projections.obligations.clone();
        let bounds = self.db.trait_bound_facts.clone();
        let mut candidates = Vec::new();
        for obligation in obligations {
            let Some(self_ty) = obligation.self_ty else {
                continue;
            };
            if let Some(trait_ty) = obligation.trait_ty {
                if logic.proves_candidate(
                    obligation.projection,
                    self_ty,
                    obligation.assoc_type,
                    trait_ty,
                ) {
                    candidates.push(ProjectionCandidate {
                        projection: obligation.projection,
                        self_ty,
                        assoc_type: obligation.assoc_type,
                        trait_ty,
                    });
                }
                continue;
            }
            for bound in &bounds {
                if logic.proves_candidate(
                    obligation.projection,
                    self_ty,
                    obligation.assoc_type,
                    bound.trait_ty,
                ) {
                    candidates.push(ProjectionCandidate {
                        projection: obligation.projection,
                        self_ty,
                        assoc_type: obligation.assoc_type,
                        trait_ty: bound.trait_ty,
                    });
                }
            }
        }
        self.db.projections.candidates.extend(candidates);
    }

    fn derive_projection_matches(&mut self) {
        let trait_members = self.trait_members();
        let mut logic = ProjectionLogic::new(self.ccx, self.db);
        logic.load_projection_matches(&trait_members);
        let candidates = self.db.projections.candidates.clone();
        let mut matches = Vec::new();
        for candidate in candidates {
            for member in trait_members
                .iter()
                .filter(|member| member.matches_candidate(candidate))
            {
                if logic.proves_match(
                    candidate.projection,
                    candidate.self_ty,
                    member.member_assoc_type,
                    candidate.trait_ty,
                ) {
                    matches.push(ProjectionMatch {
                        projection: candidate.projection,
                        self_ty: candidate.self_ty,
                        assoc_type: member.member_assoc_type,
                        trait_ty: candidate.trait_ty,
                    });
                }
            }
        }
        self.db.projections.matches.extend(matches);
    }

    fn derive_impl_self_matches(&mut self) {
        let matches = self.db.projections.matches.clone();
        let impl_facts = self.db.assoc_type_impl_facts.clone();
        let mut impl_self_matches = Vec::new();
        let mut type_bindings = Vec::new();
        for projection_match in matches {
            for impl_fact in &impl_facts {
                if projection_match.assoc_type != impl_fact.assoc_type
                    || !self.same_type(projection_match.trait_ty, impl_fact.trait_ty)
                {
                    continue;
                }
                let Some(bindings) =
                    self.type_bindings(projection_match.self_ty, impl_fact.impl_self_ty)
                else {
                    continue;
                };
                let match_ = ImplSelfMatch {
                    projection_self_ty: projection_match.self_ty,
                    impl_self_ty: impl_fact.impl_self_ty,
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
        self.db.impl_self_matches.extend(impl_self_matches);
        self.db.type_binding_facts.extend(type_bindings);
    }

    fn derive_type_substitutions(&mut self) {
        let impl_facts = self.db.assoc_type_impl_facts.clone();
        let bindings = self.db.type_binding_facts.clone();
        let mut substitutions = Vec::new();
        for impl_fact in impl_facts {
            let mut contexts = Vec::new();
            for binding in bindings
                .iter()
                .filter(|binding| binding.impl_self_ty == impl_fact.impl_self_ty)
            {
                let context = (binding.projection_self_ty, binding.impl_self_ty);
                if !contexts.contains(&context) {
                    contexts.push(context);
                }
            }

            for (projection_self_ty, impl_self_ty) in contexts {
                let impl_bindings = bindings
                    .iter()
                    .filter(|binding| {
                        binding.projection_self_ty == projection_self_ty
                            && binding.impl_self_ty == impl_self_ty
                    })
                    .copied()
                    .collect::<Vec<_>>();
                let Some((substituted_ty, used_bindings)) =
                    self.substitute_type(impl_fact.value_ty, &impl_bindings)
                else {
                    continue;
                };
                for binding in used_bindings {
                    let substitution = TypeSubstitution {
                        projection_self_ty: binding.projection_self_ty,
                        impl_self_ty: binding.impl_self_ty,
                        value_ty: impl_fact.value_ty,
                        generic_ty: binding.generic_ty,
                        arg_ty: binding.arg_ty,
                        substituted_ty,
                    };
                    if !substitutions.contains(&substitution) {
                        substitutions.push(substitution);
                    }
                }
            }
        }
        self.db.type_substitutions.extend(substitutions);
    }

    fn derive_projection_normalizations(&mut self) {
        let mut logic = ProjectionLogic::new(self.ccx, self.db);
        logic.load_projection_normalizations();
        let matches = self.db.projections.matches.clone();
        let impl_facts = self.db.assoc_type_impl_facts.clone();
        let substitutions = self.db.type_substitutions.clone();
        let mut normalizations = Vec::new();
        for projection_match in matches {
            for impl_fact in &impl_facts {
                let substituted_values = substitutions
                    .iter()
                    .filter(|substitution| {
                        substitution.projection_self_ty == projection_match.self_ty
                            && substitution.impl_self_ty == impl_fact.impl_self_ty
                            && substitution.value_ty == impl_fact.value_ty
                    })
                    .map(|substitution| substitution.substituted_ty);
                for value_ty in std::iter::once(impl_fact.value_ty).chain(substituted_values) {
                    if logic.proves_normalization(
                        projection_match.projection,
                        projection_match.self_ty,
                        projection_match.assoc_type,
                        projection_match.trait_ty,
                        value_ty,
                    ) {
                        let normalization = ProjectionNormalization {
                            projection: projection_match.projection,
                            self_ty: projection_match.self_ty,
                            assoc_type: projection_match.assoc_type,
                            trait_ty: projection_match.trait_ty,
                            value_ty,
                        };
                        if !normalizations.contains(&normalization) {
                            normalizations.push(normalization);
                        }
                    }
                }
            }
        }
        self.db.projections.normalizations.extend(normalizations);
    }

    fn trait_members(&self) -> Vec<TraitMember> {
        self.db
            .projections
            .candidates
            .iter()
            .filter_map(|candidate| self.trait_member(*candidate))
            .collect()
    }

    fn trait_member(&self, candidate: ProjectionCandidate) -> Option<TraitMember> {
        let trait_def = self.nominal_def(candidate.trait_ty)?;
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
            trait_ty: candidate.trait_ty,
            requested_assoc_type: candidate.assoc_type,
            member_assoc_type,
        })
    }

    fn nominal_def(&self, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.db.types[ty.index()] else {
            return None;
        };
        let PathTypeResolution::Nominal(def) = path.resolution else {
            return None;
        };
        Some(def)
    }

    fn same_type(&self, left: TypeId, right: TypeId) -> bool {
        left == right || self.db.types[left.index()] == self.db.types[right.index()]
    }

    fn type_bindings(
        &self,
        projection_self_ty: TypeId,
        impl_self_ty: TypeId,
    ) -> Option<Vec<TypeBindingFact>> {
        let projection_path = self.path_type(projection_self_ty)?;
        let impl_path = self.path_type(impl_self_ty)?;
        if self.nominal_def(projection_self_ty)? != self.nominal_def(impl_self_ty)? {
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
            let GenericArgument::Type(arg_ty) = projection_arg else {
                return None;
            };
            let GenericArgument::Type(generic_ty) = impl_arg else {
                return None;
            };
            self.generic_def(*generic_ty)?;
            bindings.push(TypeBindingFact {
                projection_self_ty,
                impl_self_ty,
                generic_ty: *generic_ty,
                arg_ty: *arg_ty,
            });
        }

        Some(bindings)
    }

    fn substitute_type(
        &mut self,
        ty: TypeId,
        bindings: &[TypeBindingFact],
    ) -> Option<(TypeId, Vec<TypeBindingFact>)> {
        if let Some(binding) = self.binding_for_generic(ty, bindings) {
            return Some((binding.arg_ty, vec![binding]));
        }

        match self.db.types[ty.index()].clone() {
            Type::Array { elem, len } => {
                let (elem, used) = self.substitute_type(elem, bindings)?;
                Some((self.db.intern_type(Type::Array { elem, len }), used))
            }
            Type::Infer | Type::Primitive(_) => None,
            Type::Path(path) => {
                let (path, used) = self.substitute_path_type(path, bindings)?;
                Some((self.db.intern_type(Type::Path(path)), used))
            }
            Type::Reference { elem, is_mut } => {
                let (elem, used) = self.substitute_type(elem, bindings)?;
                Some((self.db.intern_type(Type::Reference { elem, is_mut }), used))
            }
            Type::Slice { elem } => {
                let (elem, used) = self.substitute_type(elem, bindings)?;
                Some((self.db.intern_type(Type::Slice { elem }), used))
            }
            Type::Tuple { elems } => {
                let (elems, used) = self.substitute_type_ids(elems, bindings);
                if used.is_empty() {
                    return None;
                }
                Some((self.db.intern_type(Type::Tuple { elems }), used))
            }
        }
    }

    fn substitute_path_type(
        &self,
        path: PathType<'cx>,
        bindings: &[TypeBindingFact],
    ) -> Option<(PathType<'cx>, Vec<TypeBindingFact>)> {
        let mut used = Vec::new();
        let qself = path.qself.map(|qself| {
            let (self_ty, self_used) = self.substitute_type_id(qself.self_ty, bindings);
            used.extend(self_used);
            let trait_ty = qself.trait_ty.map(|trait_ty| {
                let (trait_ty, trait_used) = self.substitute_type_id(trait_ty, bindings);
                used.extend(trait_used);
                trait_ty
            });
            crate::QSelf { self_ty, trait_ty }
        });
        let segments = path
            .path
            .segments
            .into_iter()
            .map(|segment| {
                let args = segment
                    .args
                    .into_iter()
                    .map(|arg| self.substitute_generic_argument(arg, bindings, &mut used))
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
        &self,
        arg: GenericArgument<'cx>,
        bindings: &[TypeBindingFact],
        used: &mut Vec<TypeBindingFact>,
    ) -> GenericArgument<'cx> {
        match arg {
            GenericArgument::Type(ty) => {
                let (ty, ty_used) = self.substitute_type_id(ty, bindings);
                used.extend(ty_used);
                GenericArgument::Type(ty)
            }
            GenericArgument::AssocType { name, ty } => {
                let (ty, ty_used) = self.substitute_type_id(ty, bindings);
                used.extend(ty_used);
                GenericArgument::AssocType { name, ty }
            }
            GenericArgument::Const(arg) => GenericArgument::Const(arg),
            GenericArgument::AssocConst { name, value } => {
                GenericArgument::AssocConst { name, value }
            }
            GenericArgument::Constraint { name, bounds } => {
                let (bounds, bounds_used) = self.substitute_type_bounds(bounds, bindings);
                used.extend(bounds_used);
                GenericArgument::Constraint { name, bounds }
            }
            GenericArgument::Unsupported => GenericArgument::Unsupported,
        }
    }

    fn substitute_type_bounds(
        &self,
        bounds: crate::TypeBounds<'cx>,
        bindings: &[TypeBindingFact],
    ) -> (crate::TypeBounds<'cx>, Vec<TypeBindingFact>) {
        let mut used = Vec::new();
        let bounds = bounds
            .bounds
            .into_iter()
            .map(|bound| {
                let (bound, bound_used) = self.substitute_type_param_bound(bound, bindings);
                used.extend(bound_used);
                bound
            })
            .collect();
        (crate::TypeBounds { bounds }, Self::unique_bindings(used))
    }

    fn substitute_type_param_bound(
        &self,
        bound: crate::TypeParamBound<'cx>,
        bindings: &[TypeBindingFact],
    ) -> (crate::TypeParamBound<'cx>, Vec<TypeBindingFact>) {
        match bound {
            crate::TypeParamBound::Trait(bound) => {
                let (path, used) = self.substitute_path(bound.path, bindings);
                (
                    crate::TypeParamBound::Trait(crate::TraitBound { path }),
                    used,
                )
            }
            crate::TypeParamBound::Unsupported => (crate::TypeParamBound::Unsupported, Vec::new()),
        }
    }

    fn substitute_path(
        &self,
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
                        let arg = self.substitute_generic_argument(arg, bindings, &mut arg_used);
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
        &self,
        tys: Vec<TypeId>,
        bindings: &[TypeBindingFact],
    ) -> (Vec<TypeId>, Vec<TypeBindingFact>) {
        let mut used = Vec::new();
        let tys = tys
            .into_iter()
            .map(|ty| {
                let (ty, ty_used) = self.substitute_type_id(ty, bindings);
                used.extend(ty_used);
                ty
            })
            .collect();
        (tys, Self::unique_bindings(used))
    }

    fn substitute_type_id(
        &self,
        ty: TypeId,
        bindings: &[TypeBindingFact],
    ) -> (TypeId, Vec<TypeBindingFact>) {
        if let Some(binding) = self.binding_for_generic(ty, bindings) {
            return (binding.arg_ty, vec![binding]);
        }
        (ty, Vec::new())
    }

    fn binding_for_generic(
        &self,
        ty: TypeId,
        bindings: &[TypeBindingFact],
    ) -> Option<TypeBindingFact> {
        let generic_def = self.generic_def(ty)?;
        bindings
            .iter()
            .copied()
            .find(|binding| self.generic_def(binding.generic_ty) == Some(generic_def))
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

    fn path_type(&self, ty: TypeId) -> Option<&PathType<'cx>> {
        let Type::Path(path) = &self.db.types[ty.index()] else {
            return None;
        };
        Some(path)
    }

    fn generic_def(&self, ty: TypeId) -> Option<DefId> {
        let Type::Path(path) = &self.db.types[ty.index()] else {
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
    trait_ty: TypeId,
    requested_assoc_type: DefId,
    member_assoc_type: DefId,
}

impl TraitMember {
    fn matches_candidate(self, candidate: ProjectionCandidate) -> bool {
        self.trait_ty == candidate.trait_ty && self.requested_assoc_type == candidate.assoc_type
    }
}

struct ProjectionLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    infer: &'a InferDb<'cx>,
    db: Database<term::LogicAtom<'cx>>,
}

impl<'a, 'cx> ProjectionLogic<'a, 'cx> {
    fn new(ccx: &'cx CommonCx, infer: &'a InferDb<'cx>) -> Self {
        Self {
            ccx,
            infer,
            db: Database::new(),
        }
    }

    fn load_projection_candidates(&mut self) {
        self.insert_candidate_rules();
        self.insert_projection_obligations();
        self.insert_trait_bounds();
        self.insert_same_types();
        self.db.commit();
    }

    fn load_projection_matches(&mut self, trait_members: &[TraitMember]) {
        self.insert_match_rules();
        self.insert_projection_candidates();
        self.insert_trait_members(trait_members);
        self.insert_impl_assoc_types();
        self.db.commit();
    }

    fn load_projection_normalizations(&mut self) {
        self.insert_normalization_rules();
        self.insert_projection_matches();
        self.insert_impl_assoc_types();
        self.insert_impl_self_matches();
        self.insert_type_binding_facts();
        self.insert_type_substitutions();
        self.insert_same_types();
        self.db.commit();
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
        for obligation in &self.infer.projections.obligations {
            let Some(self_ty) = obligation.self_ty else {
                continue;
            };
            self.insert_clause(term::projection_obligation_clause(
                self.ccx,
                *obligation,
                self_ty,
            ));
        }
    }

    fn insert_trait_bounds(&mut self) {
        for bound in &self.infer.trait_bound_facts {
            self.insert_clause(term::trait_bound_clause(self.ccx, *bound));
        }
    }

    fn insert_same_types(&mut self) {
        for left in 0..self.infer.types.len() {
            for right in 0..self.infer.types.len() {
                let left = TypeId::new(left);
                let right = TypeId::new(right);
                if left != right
                    && self.infer.types[left.index()] != self.infer.types[right.index()]
                {
                    continue;
                }
                self.insert_clause(term::same_type_clause(self.ccx, left, right));
            }
        }
    }

    fn insert_projection_candidates(&mut self) {
        for candidate in &self.infer.projections.candidates {
            self.insert_clause(term::projection_candidate_clause(self.ccx, *candidate));
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.infer.projections.matches {
            self.insert_clause(term::projection_match_clause(self.ccx, *projection_match));
        }
    }

    fn insert_trait_members(&mut self, trait_members: &[TraitMember]) {
        for member in trait_members {
            self.insert_clause(term::trait_member_clause(
                self.ccx,
                member.trait_ty,
                member.requested_assoc_type,
                member.member_assoc_type,
            ));
        }
    }

    fn insert_impl_assoc_types(&mut self) {
        for fact in &self.infer.assoc_type_impl_facts {
            self.insert_clause(term::impl_assoc_type_clause(self.ccx, *fact));
        }
    }

    fn insert_impl_self_matches(&mut self) {
        for match_ in &self.infer.impl_self_matches {
            self.insert_clause(term::impl_self_match_clause(self.ccx, *match_));
        }
    }

    fn insert_type_binding_facts(&mut self) {
        for binding in &self.infer.type_binding_facts {
            self.insert_clause(term::type_binding_clause(self.ccx, *binding));
        }
    }

    fn insert_type_substitutions(&mut self) {
        for substitution in &self.infer.type_substitutions {
            self.insert_clause(term::type_substitution_clause(self.ccx, *substitution));
        }
    }

    fn proves_candidate(
        &mut self,
        projection: TypeId,
        self_ty: TypeId,
        assoc_type: DefId,
        trait_ty: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_candidate_query(
                self.ccx, projection, self_ty, assoc_type, trait_ty,
            ))
            .is_true()
    }

    fn proves_match(
        &mut self,
        projection: TypeId,
        self_ty: TypeId,
        assoc_type: DefId,
        trait_ty: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_match_query(
                self.ccx, projection, self_ty, assoc_type, trait_ty,
            ))
            .is_true()
    }

    fn proves_normalization(
        &mut self,
        projection: TypeId,
        self_ty: TypeId,
        assoc_type: DefId,
        trait_ty: TypeId,
        value_ty: TypeId,
    ) -> bool {
        self.db
            .query(term::projection_normalization_query(
                self.ccx, projection, self_ty, assoc_type, trait_ty, value_ty,
            ))
            .is_true()
    }

    fn insert_clause(&mut self, clause: term::LogicClause<'cx>) {
        self.db.insert_clause(clause);
    }
}
