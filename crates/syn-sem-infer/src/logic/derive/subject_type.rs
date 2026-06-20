//! Logic-backed subject type equality derivation.

use crate::{InferDb, ResolvedTypeFact, Type, TypeId, TypeSubject};
use logic_eval::Database;
use syn_sem_common::CommonCx;

use crate::logic::term;

/// Uses [`SubjectTypeLogic`] to resolve subject equality, then stores the derived type facts
/// in [`InferDb`].
pub(super) struct SubjectTypeDeriver<'a, 'cx> {
    ccx: &'cx CommonCx,
    db: &'a mut InferDb<'cx>,
}

impl<'a, 'cx> SubjectTypeDeriver<'a, 'cx> {
    pub(super) fn new(ccx: &'cx CommonCx, db: &'a mut InferDb<'cx>) -> Self {
        Self { ccx, db }
    }

    pub(super) fn derive(&mut self) {
        let resolved = self.derive_resolved_type_facts();
        self.db.subject_types.extend_resolved(resolved);
    }

    fn derive_resolved_type_facts(&mut self) -> Vec<ResolvedTypeFact> {
        let mut logic = SubjectTypeLogic::new(self.ccx, self.db);
        logic.load_subject_type_facts();

        let subjects = subject_type_subjects(self.db);
        let mut resolved = Vec::new();
        for subject in subjects {
            let candidates = logic.resolved_types(subject);
            if let Some(ty_id) = self.canonical_type(&candidates) {
                resolved.push(ResolvedTypeFact { subject, ty_id });
            }
        }
        resolved
    }

    fn canonical_type(&self, candidates: &[TypeId]) -> Option<TypeId> {
        candidates
            .iter()
            .copied()
            .try_fold(None, |selected, candidate| match selected {
                None => Some(Some(candidate)),
                Some(selected) => self.merge_candidates(selected, candidate).map(Some),
            })
            .flatten()
    }

    fn merge_candidates(&self, selected: TypeId, candidate: TypeId) -> Option<TypeId> {
        if selected == candidate || self.db[selected] == self.db[candidate] {
            return Some(selected);
        }

        let selected_primitive = self.primitive(selected);
        let candidate_primitive = self.primitive(candidate);
        match (selected_primitive, candidate_primitive) {
            (Some(selected_primitive), Some(candidate_primitive)) => {
                if selected_primitive.is_abstract_of(candidate_primitive) {
                    Some(candidate)
                } else if candidate_primitive.is_abstract_of(selected_primitive) {
                    Some(selected)
                } else {
                    None
                }
            }
            (Some(selected_primitive), _) if selected_primitive.is_abstract_numeric() => None,
            (_, Some(candidate_primitive)) if candidate_primitive.is_abstract_numeric() => None,
            _ => None,
        }
    }

    fn primitive(&self, ty_id: TypeId) -> Option<crate::PrimitiveType> {
        match &self.db[ty_id] {
            Type::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }
}

fn subject_type_subjects(db: &InferDb<'_>) -> Vec<TypeSubject> {
    let mut subjects = Vec::new();
    for fact in &db.subject_types.equalities {
        if !subjects.contains(&fact.left) {
            subjects.push(fact.left);
        }
        if !subjects.contains(&fact.right) {
            subjects.push(fact.right);
        }
    }
    subjects
}

/// Performs subject type logic operations:
///
/// * Loads equality rules
/// * Loads subject type equality facts
/// * Loads known inference type candidates
/// * Queries type candidates reachable for an inference subject
struct SubjectTypeLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    infer: &'a InferDb<'cx>,
    db: Database<term::LogicAtom<'cx>>,
}

impl<'a, 'cx> SubjectTypeLogic<'a, 'cx> {
    fn new(ccx: &'cx CommonCx, infer: &'a InferDb<'cx>) -> Self {
        Self {
            ccx,
            infer,
            db: Database::new(),
        }
    }

    fn load_subject_type_facts(&mut self) {
        for clause in term::subject_type_rules(self.ccx) {
            self.insert_clause(clause);
        }
        for fact in &self.infer.subject_types.equalities {
            self.insert_clause(term::type_equal_clause(self.ccx, *fact));
        }
        for (ty_id, ty) in self.infer.types.iter() {
            if matches!(ty, Type::Infer) {
                continue;
            }
            self.insert_clause(term::type_candidate_clause(self.ccx, ty_id));
        }
        self.db.commit();
    }

    fn resolved_types(&mut self, subject: TypeSubject) -> Vec<TypeId> {
        let mut query = self.db.query(term::resolved_type_query(self.ccx, subject));
        let mut types = Vec::new();
        while let Some(result) = query.prove_next() {
            for assignment in result {
                if assignment.get_lhs_variable().as_ref() != "$Type" {
                    continue;
                }
                let ty = assignment.rhs();
                let Some(ty) = term::type_id_from_term(&ty) else {
                    continue;
                };
                if !types.contains(&ty) {
                    types.push(ty);
                }
            }
        }
        types
    }

    fn insert_clause(&mut self, clause: term::LogicClause<'cx>) {
        self.db.insert_clause(clause);
    }
}
