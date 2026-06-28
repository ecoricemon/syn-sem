//! Type relation equality facts and resolved lookup maps.

use crate::TypeId;
use syn_sem_common::{Map, VecUniqueExt};
use syn_sem_hir as hir;
use syn_sem_name as name;

/// Type relations between inference subjects.
#[derive(Debug, Default)]
pub(crate) struct TypeRelationDb {
    /// Equality relations between inference subjects.
    /// * Made in the build stage.
    pub(crate) equalities: Vec<TypeEqualityFact>,
    /// Type resolutions derived from equality relations.
    /// * Made in the derive stage.
    pub(crate) resolved: Vec<ResolvedTypeFact>,
    /// Lookup map derived from [`Self::resolved`] for HIR expression occurrences.
    /// * Made in the derive stage.
    pub(crate) expr_types: Map<hir::ExprId, TypeId>,
    /// Lookup map derived from [`Self::resolved`] for definitions.
    /// * Made in the derive stage.
    pub(crate) def_types: Map<name::DefId, TypeId>,
}

impl TypeRelationDb {
    /// Returns the resolved type linked to a HIR expression occurrence.
    pub(crate) fn type_for_hir_expr(&self, hir_expr: hir::ExprId) -> Option<TypeId> {
        self.expr_types.get(&hir_expr).copied()
    }

    /// Returns the resolved type linked to a definition.
    pub(crate) fn type_for_def(&self, def: name::DefId) -> Option<TypeId> {
        self.def_types.get(&def).copied()
    }

    /// Records resolved type mappings derived from equality relations.
    pub(crate) fn extend_resolved(&mut self, resolved: Vec<ResolvedTypeFact>) {
        for fact in &resolved {
            match fact.subject {
                TypeSubject::Def(def) => {
                    let previous = self.def_types.insert(def, fact.ty);
                    assert!(previous.is_none(), "resolved def type must be unique");
                }
                TypeSubject::Expr(expr) => {
                    let previous = self.expr_types.insert(expr, fact.ty);
                    assert!(
                        previous.is_none(),
                        "resolved expression type must be unique"
                    );
                }
                TypeSubject::Type(_) => {}
            }
        }
        self.resolved.extend(resolved);
    }

    /// Adds an equality relation between inference subjects.
    pub(crate) fn insert_equality(&mut self, fact: TypeEqualityFact) -> bool {
        self.equalities.push_unique(fact)
    }

    /// Clears resolved type mappings before recomputing them from equality relations.
    pub(crate) fn clear_resolved(&mut self) {
        self.resolved.clear();
        self.expr_types.clear();
        self.def_types.clear();
    }

    /// Returns equality relations between inference subjects.
    #[cfg(test)]
    pub(crate) fn equalities(&self) -> &[TypeEqualityFact] {
        &self.equalities
    }

    /// Returns type resolutions derived from equality relations.
    #[cfg(test)]
    pub(crate) fn resolved(&self) -> &[ResolvedTypeFact] {
        &self.resolved
    }
}

/// Equality relation between two inference subjects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TypeEqualityFact {
    /// Left side of the equality edge.
    pub(crate) left: TypeSubject,
    /// Right side of the equality edge.
    pub(crate) right: TypeSubject,
}

/// Resolved type found for an inference subject.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedTypeFact {
    /// Subject being resolved.
    pub(crate) subject: TypeSubject,
    /// Inference type selected for the subject through equality edges.
    pub(crate) ty: TypeId,
}

/// Subject whose type can participate in type relation resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum TypeSubject {
    /// A definition such as a parameter or local binding.
    Def(name::DefId),
    /// A HIR expression occurrence.
    Expr(hir::ExprId),
    /// An inference type.
    Type(TypeId),
}
