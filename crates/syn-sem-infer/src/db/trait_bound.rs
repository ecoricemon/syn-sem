//! Trait-bound fact collection for inference.

use super::infer_types::{InferTypes, TypeLowerer};
use crate::TraitBoundFact;
use syn_sem_hir as hir;
use syn_sem_name::NameDb;

pub(super) struct TraitBoundFactCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    lowerer: TypeLowerer<'a, 'cx>,
    facts: Vec<TraitBoundFact>,
}

impl<'a, 'cx> TraitBoundFactCollector<'a, 'cx> {
    pub(super) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<TraitBoundFact> {
        Self {
            hir,
            lowerer: TypeLowerer::new(hir, names, types),
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
            let hir::WherePredicate::TypeBound { subject, bounds } = predicate else {
                continue;
            };
            let subject = self.lowerer.lower_hir_type(*subject);
            for bound in bounds {
                let hir::TypeParamBound::Trait(bound) = bound else {
                    continue;
                };
                let trait_ty = self
                    .lowerer
                    .lower_path_value_as_type(&bound.path, generics.scope);
                self.facts.push(TraitBoundFact { subject, trait_ty });
            }
        }
    }
}
