//! Logic-backed body-local type equality derivation.

use crate::{InferDb, ResolvedTypeFact, Type, TypeId, TypeSubject};
use logic_eval::Database;
use syn_sem_common::CommonCx;

use crate::logic::term;

/// Uses [`BodyTypeLogic`] to resolve body-local equality subjects, then stores the derived concrete
/// type facts in [`InferDb`].
pub(super) struct BodyTypeDeriver<'a, 'cx> {
    ccx: &'cx CommonCx,
    db: &'a mut InferDb<'cx>,
}

impl<'a, 'cx> BodyTypeDeriver<'a, 'cx> {
    pub(super) fn new(ccx: &'cx CommonCx, db: &'a mut InferDb<'cx>) -> Self {
        Self { ccx, db }
    }

    pub(super) fn derive(&mut self) {
        let resolved = self.derive_resolved_type_facts();
        self.db.body_types.extend_resolved(resolved);
    }

    fn derive_resolved_type_facts(&mut self) -> Vec<ResolvedTypeFact> {
        let mut logic = BodyTypeLogic::new(self.ccx, self.db);
        logic.load_body_type_facts();

        let subjects = body_type_subjects(self.db);
        let mut resolved = Vec::new();
        for subject in subjects {
            for ty in logic.resolved_types(subject) {
                let fact = ResolvedTypeFact { subject, ty };
                if !resolved.contains(&fact) {
                    resolved.push(fact);
                }
            }
        }
        resolved
    }
}

fn body_type_subjects(db: &InferDb<'_>) -> Vec<TypeSubject> {
    let mut subjects = Vec::new();
    for fact in &db.body_types.equalities {
        if !subjects.contains(&fact.left) {
            subjects.push(fact.left);
        }
        if !subjects.contains(&fact.right) {
            subjects.push(fact.right);
        }
    }
    subjects
}

/// Performs body type logic operations:
///
/// * Loads equality rules
/// * Loads body-local type equality facts
/// * Loads known concrete inference types
/// * Queries concrete types resolved for a body-local subject
struct BodyTypeLogic<'a, 'cx> {
    ccx: &'cx CommonCx,
    infer: &'a InferDb<'cx>,
    db: Database<term::LogicAtom<'cx>>,
}

impl<'a, 'cx> BodyTypeLogic<'a, 'cx> {
    fn new(ccx: &'cx CommonCx, infer: &'a InferDb<'cx>) -> Self {
        Self {
            ccx,
            infer,
            db: Database::new(),
        }
    }

    fn load_body_type_facts(&mut self) {
        for clause in term::body_type_rules(self.ccx) {
            self.insert_clause(clause);
        }
        for fact in &self.infer.body_types.equalities {
            self.insert_clause(term::type_equal_clause(self.ccx, *fact));
        }
        for (index, ty) in self.infer.types.iter().enumerate() {
            if matches!(ty, Type::Infer) {
                continue;
            }
            self.insert_clause(term::concrete_type_clause(self.ccx, TypeId::new(index)));
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
