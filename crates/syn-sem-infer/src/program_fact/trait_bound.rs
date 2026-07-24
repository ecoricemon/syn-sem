//! Trait-bound collection for inference.

use crate::{InferTypes, TypeId, TypeLowerer};
use syn_sem_hir as hir;
use syn_sem_name::NameDb;

pub(crate) struct TraitBoundCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    ty_lowerer: TypeLowerer<'a, 'cx>,
    trait_bounds: Vec<TraitBound>,
}

impl<'a, 'cx> TraitBoundCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<TraitBound> {
        Self {
            hir,
            ty_lowerer: TypeLowerer::new(hir, names, types),
            trait_bounds: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> Vec<TraitBound> {
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
        self.trait_bounds
    }

    fn collect_generics(&mut self, generics: &hir::Generics<'cx>) {
        for predicate in &generics.predicates {
            let hir::WherePredicate::TypeBound { subject, bounds } = predicate else {
                continue;
            };
            let subject = self.ty_lowerer.lower_hir_type(*subject);
            for bound in bounds {
                let hir::TypeParamBound::Trait(path) = bound else {
                    continue;
                };
                let trait_ = self
                    .ty_lowerer
                    .lower_plain_path_as_type(&path.segments, generics.scope);
                self.trait_bounds.push(TraitBound { subject, trait_ });
            }
        }
    }
}

/// One trait bound collected as solver input.
///
/// A type-bound predicate can contain multiple bounds, such as `T: Debug + Clone`;
/// inference flattens that into one fact per trait bound, e.g. `T: Debug` and `T: Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TraitBound {
    /// Type constrained by the trait bound.
    pub(crate) subject: TypeId,
    /// Trait type required by the bound.
    pub(crate) trait_: TypeId,
}
