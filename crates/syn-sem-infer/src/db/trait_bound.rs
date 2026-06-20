//! Trait-bound fact collection for inference.

use super::infer_types::{InferTypes, TypeLowerer};
use crate::TypeId;
use syn_sem_hir as hir;
use syn_sem_name::NameDb;

pub(crate) struct TraitBoundFactCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    ty_lowerer: TypeLowerer<'a, 'cx>,
    facts: Vec<TraitBoundFact>,
}

impl<'a, 'cx> TraitBoundFactCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<TraitBoundFact> {
        Self {
            hir,
            ty_lowerer: TypeLowerer::new(hir, names, types),
            facts: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> Vec<TraitBoundFact> {
        for item in self.hir.items() {
            match &item.kind {
                hir::ItemKind::Enum { generics, .. }
                | hir::ItemKind::Fn { generics, .. }
                | hir::ItemKind::Impl { generics, .. }
                | hir::ItemKind::Struct { generics, .. }
                | hir::ItemKind::Trait { generics, .. }
                | hir::ItemKind::Type { generics, .. } => {
                    self.collect_generics(generics);
                }
                hir::ItemKind::Const { .. }
                | hir::ItemKind::Mod { .. }
                | hir::ItemKind::Use { .. } => {}
            }
        }
        self.facts
    }

    fn collect_generics(&mut self, generics: &hir::Generics<'cx>) {
        for predicate in &generics.predicates {
            let hir::WherePredicate::TypeBound {
                subject_ty_id,
                bounds,
            } = predicate
            else {
                continue;
            };
            let subject_ty_id = self.ty_lowerer.lower_hir_type(*subject_ty_id);
            for bound in bounds {
                let hir::TypeParamBound::Trait(path) = bound else {
                    continue;
                };
                let trait_ty_id = self
                    .ty_lowerer
                    .lower_plain_path_as_type(path, generics.scope);
                self.facts.push(TraitBoundFact {
                    subject_ty_id,
                    trait_ty_id,
                });
            }
        }
    }
}

/// One trait bound fact collected as solver input.
///
/// A type-bound predicate can contain multiple bounds, such as `T: Debug + Clone`;
/// inference flattens that into one fact per trait bound, e.g. `T: Debug` and `T: Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TraitBoundFact {
    /// Type constrained by the trait bound.
    pub(crate) subject_ty_id: TypeId,
    /// Trait type required by the bound.
    pub(crate) trait_ty_id: TypeId,
}
