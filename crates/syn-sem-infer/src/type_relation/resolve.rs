//! Logic-backed type relation resolution.

use super::term as relation_term;
use super::{ResolvedTypeFact, TypeRelationDb, TypeSubject};
use crate::{
    logic::{self, symbol::var},
    InferTypes, PrimitiveType, Type, TypeId,
};
use logic_eval::Database;
use syn_sem_common::{CommonCx, Set, VecUniqueExt};

/// Resolves the type of each inference subject through collected equality relations.
///
/// A subject is something that can be linked to a type during inference: a definition
/// ([`TypeSubject::Def`]), a HIR expression occurrence ([`TypeSubject::Expr`]), or an already
/// lowered inference type ([`TypeSubject::Type`]). This resolver does not build new type shapes.
/// Instead, it follows equality edges between subjects, finds reachable [`TypeId`] candidates,
/// chooses one canonical type for each subject, and stores the resolved lookup facts.
pub(crate) struct TypeRelationResolver<'a, 'cx> {
    type_relations: &'a mut TypeRelationDb,
    ccx: &'cx CommonCx,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> TypeRelationResolver<'a, 'cx> {
    pub(crate) fn new(
        type_relations: &'a mut TypeRelationDb,
        ccx: &'cx CommonCx,
        types: &'a InferTypes<'cx>,
    ) -> Self {
        Self {
            type_relations,
            ccx,
            types,
        }
    }

    /// Runs the type relation resolution pipeline.
    ///
    /// For example, given:
    /// ```text
    /// fn f(x: usize) -> usize {
    ///     let y = x;
    ///     y
    /// }
    /// ```
    ///
    /// this derives:
    /// ```text
    /// equality facts:      Def(x) == Type(usize)
    ///                      Def(y) == Expr(x)
    ///                      Expr(tail y) == Def(y)
    ///                      Expr(tail y) == Type(return usize)
    /// reachable candidate: Type(usize)
    /// resolved subjects:   Def(x) -> usize
    ///                      Expr(x) -> usize
    ///                      Def(y) -> usize
    ///                      Expr(tail y) -> usize
    /// lookup maps:         type_for_def(y) and type_for_hir_expr(tail y) return usize
    /// ```
    ///
    /// If a subject reaches both an abstract numeric literal type and a compatible concrete
    /// primitive, such as `AbstractInt` and `i32` for `let a: i32 = 1`, the concrete primitive is
    /// selected. Incompatible candidates are left unresolved.
    pub(crate) fn resolve(&mut self) {
        let resolved = self.resolve_type_facts();
        self.type_relations.extend_resolved(resolved);
    }

    fn resolve_type_facts(&mut self) -> Vec<ResolvedTypeFact> {
        let mut logic = TypeRelationLogic::new(self.ccx, self.type_relations, self.types);
        logic.load_type_relation_facts();

        let mut seen_subjects = Set::default();
        self.type_relations
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

/// Performs type relation logic operations:
///
/// * Loads equality rules
/// * Loads type relation equality facts
/// * Loads known inference type candidates
/// * Queries type candidates reachable for an inference subject
struct TypeRelationLogic<'a, 'cx> {
    db: Database<logic::LogicAtom<'cx>>,
    ccx: &'cx CommonCx,
    type_relations: &'a TypeRelationDb,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> TypeRelationLogic<'a, 'cx> {
    fn new(
        ccx: &'cx CommonCx,
        type_relations: &'a TypeRelationDb,
        types: &'a InferTypes<'cx>,
    ) -> Self {
        Self {
            db: Database::default(),
            ccx,
            type_relations,
            types,
        }
    }

    fn load_type_relation_facts(&mut self) {
        for clause in relation_term::type_relation_rules(self.ccx) {
            self.db.insert_clause(clause);
        }
        for fact in &self.type_relations.equalities {
            self.db
                .insert_clause(relation_term::type_equality_clause(self.ccx, *fact));
        }
        for (ty, _) in self
            .types
            .iter()
            .filter(|(_, ty)| !matches!(ty, Type::Infer))
        {
            self.db
                .insert_clause(relation_term::type_candidate_clause(self.ccx, ty));
        }
    }

    fn resolved_types(&mut self, subject: TypeSubject) -> Vec<TypeId> {
        let mut query = self
            .db
            .query(relation_term::resolved_type_query(self.ccx, subject));
        let mut types = Vec::new();
        while let Some(answer) = query.prove_next() {
            let Some(ty) = answer
                .get(var::TYPE)
                .and_then(|term| logic::type_id_from_term(&term))
            else {
                continue;
            };
            types.push_unique(ty);
        }
        types
    }
}
