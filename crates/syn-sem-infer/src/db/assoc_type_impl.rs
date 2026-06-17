//! Associated type implementation fact collection for inference.

use super::infer_types::{InferTypes, TypeLowerer};
use crate::AssocTypeImplFact;
use syn_sem_hir as hir;
use syn_sem_name::{NameDb, Namespace, ResolveResult};

pub(super) struct AssocTypeImplFactCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    lowerer: TypeLowerer<'a, 'cx>,
    facts: Vec<AssocTypeImplFact>,
}

impl<'a, 'cx> AssocTypeImplFactCollector<'a, 'cx> {
    pub(super) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<AssocTypeImplFact> {
        Self {
            hir,
            names,
            lowerer: TypeLowerer::new(hir, names, types),
            facts: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> Vec<AssocTypeImplFact> {
        for item in self.hir.items() {
            let hir::ItemKind::Impl {
                trait_,
                self_ty,
                items,
                ..
            } = &item.kind
            else {
                continue;
            };
            let Some(trait_) = trait_ else {
                continue;
            };
            let impl_self_ty = self.lowerer.lower_hir_type(*self_ty);
            let trait_ty = self
                .lowerer
                .lower_path_value_as_type(trait_, item.parent_scope);
            let Some(trait_def) = self.lowerer.trait_def_for_type(trait_ty) else {
                continue;
            };
            for assoc_item in items.iter().map(|id| &self.hir[*id]) {
                let hir::AssocItemKind::ImplType { ty } = assoc_item.kind else {
                    continue;
                };
                let ResolveResult::Found(assoc_type) =
                    self.names
                        .member(trait_def, Namespace::Type, assoc_item.name)
                else {
                    continue;
                };
                let value_ty = self.lowerer.lower_hir_type(ty);
                self.facts.push(AssocTypeImplFact {
                    impl_self_ty,
                    trait_ty,
                    assoc_type,
                    value_ty,
                });
            }
        }
        self.facts
    }
}
