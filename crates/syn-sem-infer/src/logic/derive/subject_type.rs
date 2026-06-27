//! Logic-backed subject type equality derivation.

use crate::{
    logic::term::{self, symbol::var},
    InferTypes, PrimitiveType, ResolvedTypeFact, SubjectTypeDb, Type, TypeId, TypeSubject,
};
use logic_eval::Database;
use syn_sem_common::{CommonCx, Set, VecUniqueExt};

/// Uses [`SubjectTypeLogic`] to resolve subject equality, then stores the derived type facts.
pub(crate) struct SubjectTypeDeriver<'a, 'cx> {
    subject_types: &'a mut SubjectTypeDb,
    ccx: &'cx CommonCx,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> SubjectTypeDeriver<'a, 'cx> {
    pub(crate) fn new(
        subject_types: &'a mut SubjectTypeDb,
        ccx: &'cx CommonCx,
        types: &'a InferTypes<'cx>,
    ) -> Self {
        Self {
            subject_types,
            ccx,
            types,
        }
    }

    pub(crate) fn derive(&mut self) {
        let resolved = self.derive_resolved_type_facts();
        self.subject_types.extend_resolved(resolved);
    }

    fn derive_resolved_type_facts(&mut self) -> Vec<ResolvedTypeFact> {
        let mut logic = SubjectTypeLogic::new(self.ccx, self.subject_types, self.types);
        logic.load_subject_type_facts();

        let mut seen_subjects = Set::default();
        self.subject_types
            .equalities
            .iter()
            .flat_map(|equal_fact| [equal_fact.left, equal_fact.right])
            .filter_map(|subject| {
                if !seen_subjects.insert(subject) {
                    return None;
                }
                let candidates = logic.resolved_types(subject);
                self.canonical_type(&candidates)
                    .map(|ty| ResolvedTypeFact { subject, ty })
            })
            .collect()
    }

    fn canonical_type(&self, candidates: &[TypeId]) -> Option<TypeId> {
        let mut selected = None;
        for candidate in candidates {
            selected = Some(match selected {
                None => *candidate,
                Some(selected) => self.merge_candidates(selected, *candidate)?,
            });
        }
        selected
    }

    fn merge_candidates(&self, selected: TypeId, candidate: TypeId) -> Option<TypeId> {
        if selected == candidate || self.types[selected] == self.types[candidate] {
            return Some(selected);
        }

        let selected_primitive = self.primitive(selected)?;
        let candidate_primitive = self.primitive(candidate)?;
        if selected_primitive.is_abstract_of(candidate_primitive) {
            Some(candidate)
        } else if candidate_primitive.is_abstract_of(selected_primitive) {
            Some(selected)
        } else {
            None
        }
    }

    fn primitive(&self, id: TypeId) -> Option<PrimitiveType> {
        match &self.types[id] {
            Type::Primitive(primitive) => Some(*primitive),
            _ => None,
        }
    }
}

/// Performs subject type logic operations:
///
/// * Loads equality rules
/// * Loads subject type equality facts
/// * Loads known inference type candidates
/// * Queries type candidates reachable for an inference subject
struct SubjectTypeLogic<'a, 'cx> {
    db: Database<term::LogicAtom<'cx>>,
    ccx: &'cx CommonCx,
    subject_types: &'a SubjectTypeDb,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> SubjectTypeLogic<'a, 'cx> {
    fn new(
        ccx: &'cx CommonCx,
        subject_types: &'a SubjectTypeDb,
        types: &'a InferTypes<'cx>,
    ) -> Self {
        Self {
            db: Database::default(),
            ccx,
            subject_types,
            types,
        }
    }

    fn load_subject_type_facts(&mut self) {
        for clause in term::subject_type_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
        for fact in &self.subject_types.equalities {
            self.db
                .insert_clause(term::type_equal_clause(self.ccx, *fact));
        }
        for (ty, _) in self
            .types
            .iter()
            .filter(|(_, ty)| !matches!(ty, Type::Infer))
        {
            self.db
                .insert_clause(term::type_candidate_clause(self.ccx, ty));
        }
    }

    fn resolved_types(&mut self, subject: TypeSubject) -> Vec<TypeId> {
        let mut query = self.db.query(term::resolved_type_query(self.ccx, subject));
        let mut types = Vec::new();
        while let Some(answer) = query.prove_next() {
            let Some(ty) = answer
                .get(var::TYPE)
                .and_then(|term| term::type_id_from_term(&term))
            else {
                continue;
            };
            types.push_unique(ty);
        }
        types
    }
}
