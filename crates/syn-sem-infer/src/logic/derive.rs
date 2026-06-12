use super::term;
use crate::{
    InferDb, PathTypeResolution, ProjectionCandidate, ProjectionMatch, ProjectionNormalization,
    Type, TypeId,
};
use logic_eval::Database;
use syn_sem_common::CommonCx;
use syn_sem_name::{DefId, DefKind, NameDb, Namespace, ResolveResult};

pub(crate) fn derive<'cx>(ccx: &'cx CommonCx, db: &mut InferDb<'cx>, names: &NameDb<'cx>) {
    let mut logic = LogicCx { ccx, db, names };
    logic.derive_projection_candidates();
    logic.derive_projection_matches();
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
        let obligations = self.db.projection_obligations.clone();
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
        self.db.projection_candidates.extend(candidates);
    }

    fn derive_projection_matches(&mut self) {
        let trait_members = self.trait_members();
        let mut logic = ProjectionLogic::new(self.ccx, self.db);
        logic.load_projection_matches(&trait_members);
        let candidates = self.db.projection_candidates.clone();
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
        self.db.projection_matches.extend(matches);
    }

    fn derive_projection_normalizations(&mut self) {
        let mut logic = ProjectionLogic::new(self.ccx, self.db);
        logic.load_projection_normalizations();
        let matches = self.db.projection_matches.clone();
        let impl_facts = self.db.assoc_type_impl_facts.clone();
        let mut normalizations = Vec::new();
        for projection_match in matches {
            for impl_fact in &impl_facts {
                if logic.proves_normalization(
                    projection_match.projection,
                    projection_match.self_ty,
                    projection_match.assoc_type,
                    projection_match.trait_ty,
                    impl_fact.value_ty,
                ) {
                    let normalization = ProjectionNormalization {
                        projection: projection_match.projection,
                        self_ty: projection_match.self_ty,
                        assoc_type: projection_match.assoc_type,
                        trait_ty: projection_match.trait_ty,
                        value_ty: impl_fact.value_ty,
                    };
                    if !normalizations.contains(&normalization) {
                        normalizations.push(normalization);
                    }
                }
            }
        }
        self.db.projection_normalizations.extend(normalizations);
    }

    fn trait_members(&self) -> Vec<TraitMember> {
        self.db
            .projection_candidates
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
        for obligation in &self.infer.projection_obligations {
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
        for candidate in &self.infer.projection_candidates {
            self.insert_clause(term::projection_candidate_clause(self.ccx, *candidate));
        }
    }

    fn insert_projection_matches(&mut self) {
        for projection_match in &self.infer.projection_matches {
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
