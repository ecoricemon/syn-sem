//! Analysis-scoped ownership for shared and phase-specific logic state.

use super::{type_class_clause, type_class_id, type_id, Atom};
use crate::{projection::ProjectionLogic, InferTypes, Type, TypeClassId};
use logic_eval::Database;
use syn_sem_common::Map;

/// Keeps the shared logic database paired with every adapter state during one analysis.
pub(crate) struct LogicSession<'cx> {
    logic: InferLogic<'cx>,
    projection: ProjectionLogic<'cx>,
}

impl<'cx> Default for LogicSession<'cx> {
    fn default() -> Self {
        let token = LogicSessionToken(());
        Self {
            logic: InferLogic::new(&token),
            projection: ProjectionLogic::new(&token),
        }
    }
}

impl<'cx> LogicSession<'cx> {
    /// Runs one operation with the paired database and projection synchronization state.
    pub(crate) fn with_projection<R>(
        &mut self,
        f: impl FnOnce(&mut InferLogic<'cx>, &mut ProjectionLogic<'cx>) -> R,
    ) -> R {
        f(&mut self.logic, &mut self.projection)
    }
}

/// Capability required to construct logic state owned by a [`LogicSession`].
///
/// In other words, types that require this token at their constructor cannot be created outside
/// `LogicSession`.
pub(crate) struct LogicSessionToken(());

/// Owns the logic database shared by inference logic adapters during one analysis.
pub(crate) struct InferLogic<'cx> {
    pub(crate) db: Database<Atom<'cx>>,
    type_classes: Map<Type<'cx>, TypeClassId>,
    types_cursor: InferTypesCursor,
}

impl<'cx> InferLogic<'cx> {
    pub(crate) fn new(_: &LogicSessionToken) -> Self {
        Self {
            db: Database::default(),
            type_classes: Map::default(),
            types_cursor: InferTypesCursor::default(),
        }
    }

    /// Synchronizes newly appended inference types as structural-class facts.
    pub(crate) fn sync_type_classes(&mut self, types: &InferTypes<'cx>) {
        for (ty, value) in types.iter().skip(self.types_cursor.len) {
            let class = match self.type_classes.get(value) {
                Some(class) => *class,
                None => {
                    let class = TypeClassId::new(self.type_classes.len());
                    self.type_classes.insert(value.clone(), class);
                    class
                }
            };
            self.db
                .insert_clause(type_class_clause(type_id(ty), type_class_id(class)));
        }
        self.types_cursor.len = types.len();
    }
}

/// Tracks the prefix of inference types synchronized into shared logic.
#[derive(Default)]
struct InferTypesCursor {
    len: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logic::{same_type, same_type_rules, type_id, Expr};
    use crate::{PrimitiveType, Type};

    #[test]
    fn synchronizes_all_stored_type_shapes_into_structural_classes() {
        let mut types = InferTypes::default();
        let usize_a = types.insert_fresh_type(Type::Primitive(PrimitiveType::Usize));
        let usize_b = types.insert_fresh_type(Type::Primitive(PrimitiveType::Usize));
        let bool_ = types.insert_fresh_type(Type::Primitive(PrimitiveType::Bool));
        let ref_a = types.insert_fresh_type(Type::Reference {
            elem: usize_a,
            is_mut: false,
        });
        let ref_b = types.insert_fresh_type(Type::Reference {
            elem: usize_a,
            is_mut: false,
        });
        let ref_other_id = types.insert_fresh_type(Type::Reference {
            elem: usize_b,
            is_mut: false,
        });
        let infer_a = types.insert_fresh_type(Type::Infer);
        let infer_b = types.insert_fresh_type(Type::Infer);

        LogicSession::default().with_projection(|logic, _| {
            for clause in same_type_rules() {
                logic.db.insert_clause(clause);
            }
            logic.sync_type_classes(&types);

            assert_eq!(
                logic.type_classes[&types[usize_a]],
                logic.type_classes[&types[usize_b]]
            );
            assert_ne!(
                logic.type_classes[&types[usize_a]],
                logic.type_classes[&types[bool_]]
            );
            assert_eq!(
                logic.type_classes[&types[ref_a]],
                logic.type_classes[&types[ref_b]]
            );
            assert_ne!(
                logic.type_classes[&types[ref_a]],
                logic.type_classes[&types[ref_other_id]]
            );
            assert_eq!(
                logic.type_classes[&types[infer_a]],
                logic.type_classes[&types[infer_b]]
            );

            let mut same = logic
                .db
                .query(Expr::Term(same_type(type_id(usize_a), type_id(usize_b))));
            assert!(same.prove_next().is_some());
            drop(same);

            let mut different = logic
                .db
                .query(Expr::Term(same_type(type_id(usize_a), type_id(bool_))));
            assert!(different.prove_next().is_none());
            drop(different);

            let clause_count = logic.db.clauses().count();
            let tuple_a = types.insert_fresh_type(Type::Tuple {
                elems: vec![usize_a, bool_],
            });
            let tuple_b = types.insert_fresh_type(Type::Tuple {
                elems: vec![usize_a, bool_],
            });
            logic.sync_type_classes(&types);

            assert_eq!(logic.db.clauses().count(), clause_count + 2);
            assert_eq!(
                logic.type_classes[&types[tuple_a]],
                logic.type_classes[&types[tuple_b]]
            );
        });
    }
}
