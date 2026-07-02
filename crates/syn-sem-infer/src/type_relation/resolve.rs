//! Graph-backed type relation resolution.

use super::{ResolvedTypeFact, TypeRelationDb, TypeSubject};
use crate::{InferTypes, PrimitiveType, Type, TypeId};
use syn_sem_common::{Map, VecUniqueExt};

/// Resolves the type of each inference subject through collected equality relations.
///
/// A subject is something that can be linked to a type during inference: a definition
/// ([`TypeSubject::Def`]), a HIR expression occurrence ([`TypeSubject::Expr`]), or an already
/// lowered inference type ([`TypeSubject::Type`]). This resolver does not build new type shapes.
/// Instead, it follows equality edges between subjects, finds reachable [`TypeId`] candidates,
/// chooses one canonical type for each subject, and stores the resolved lookup facts.
pub(crate) struct TypeRelationResolver<'a, 'cx> {
    type_relations: &'a mut TypeRelationDb,
    types: &'a InferTypes<'cx>,
}

impl<'a, 'cx> TypeRelationResolver<'a, 'cx> {
    pub(crate) fn new(type_relations: &'a mut TypeRelationDb, types: &'a InferTypes<'cx>) -> Self {
        Self {
            type_relations,
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

    fn resolve_type_facts(&self) -> Vec<ResolvedTypeFact> {
        let graph = TypeRelationGraph::from_equalities(&self.type_relations.equalities);
        let component_types = graph.component_types(self.types);
        graph
            .subjects()
            .iter()
            .filter_map(|subject| {
                let candidates = component_types.get(&graph.root(subject))?;
                self.unified_type(candidates).map(|ty| ResolvedTypeFact {
                    subject: *subject,
                    ty,
                })
            })
            .collect()
    }

    fn unified_type(&self, candidates: &[TypeId]) -> Option<TypeId> {
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

struct TypeRelationGraph {
    subjects: Vec<TypeSubject>,
    indexes: Map<TypeSubject, usize>,
    parents: Vec<usize>,
}

impl TypeRelationGraph {
    fn from_equalities(equalities: &[super::TypeEqualityFact]) -> Self {
        let mut graph = Self {
            subjects: Vec::new(),
            indexes: Map::default(),
            parents: Vec::new(),
        };
        for fact in equalities {
            graph.union(fact.left, fact.right);
        }
        for index in 0..graph.parents.len() {
            graph.compress(index);
        }
        graph
    }

    fn subjects(&self) -> &[TypeSubject] {
        &self.subjects
    }

    fn component_types(&self, types: &InferTypes<'_>) -> Map<usize, Vec<TypeId>> {
        let mut component_types = Map::default();
        for subject in &self.subjects {
            let TypeSubject::Type(ty) = subject else {
                continue;
            };
            // Match the old logic resolver: fresh inference placeholders are equality edges, not
            // concrete candidates that can determine a component's resolved type.
            if matches!(types[*ty], Type::Infer) {
                continue;
            }
            component_types
                .entry(self.root(subject))
                .or_insert_with(Vec::new)
                .push_unique(*ty);
        }
        component_types
    }

    fn union(&mut self, left: TypeSubject, right: TypeSubject) {
        let left = self.intern(left);
        let right = self.intern(right);
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parents[right_root] = left_root;
        }
    }

    fn compress(&mut self, index: usize) -> usize {
        let parent = self.parents[index];
        if parent == index {
            return index;
        }
        let root = self.compress(parent);
        self.parents[index] = root;
        root
    }

    fn find(&self, index: usize) -> usize {
        let mut root = index;
        while self.parents[root] != root {
            root = self.parents[root];
        }
        root
    }

    fn intern(&mut self, subject: TypeSubject) -> usize {
        if let Some(index) = self.indexes.get(&subject) {
            return *index;
        }
        let index = self.subjects.len();
        self.subjects.push(subject);
        self.indexes.insert(subject, index);
        self.parents.push(index);
        index
    }

    fn root(&self, subject: &TypeSubject) -> usize {
        self.find(self.indexes[subject])
    }
}
