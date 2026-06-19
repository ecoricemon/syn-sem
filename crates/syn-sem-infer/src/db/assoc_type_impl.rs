//! Associated type implementation fact collection for inference.

use super::infer_types::{InferTypes, TypeLowerer};
use crate::TypeId;
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb, Namespace, ResolveResult};

pub(crate) struct AssocTypeImplFactCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    ty_lowerer: TypeLowerer<'a, 'cx>,
    facts: Vec<AssocTypeImplFact>,
}

impl<'a, 'cx> AssocTypeImplFactCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<AssocTypeImplFact> {
        Self {
            hir,
            names,
            ty_lowerer: TypeLowerer::new(hir, names, types),
            facts: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> Vec<AssocTypeImplFact> {
        for item in self.hir.items() {
            let hir::ItemKind::Impl {
                trait_,
                self_tid,
                items,
                ..
            } = &item.kind
            else {
                continue;
            };
            let Some(trait_) = trait_ else {
                continue;
            };
            let impl_self_tid = self.ty_lowerer.lower_hir_type(*self_tid);
            let trait_tid = self
                .ty_lowerer
                .lower_plain_path_as_type(trait_, item.parent_scope);
            let Some(trait_def) = self.ty_lowerer.trait_def_for_type(trait_tid) else {
                continue;
            };
            for assoc_item in items.iter().map(|id| &self.hir[*id]) {
                let hir::AssocItemKind::ImplType { tid } = assoc_item.kind else {
                    continue;
                };
                let ResolveResult::Found(assoc_type) =
                    self.names
                        .member(trait_def, Namespace::Type, assoc_item.name)
                else {
                    continue;
                };
                let value_tid = self.ty_lowerer.lower_hir_type(tid);
                self.facts.push(AssocTypeImplFact {
                    impl_self_tid,
                    trait_tid,
                    assoc_type,
                    value_tid,
                });
            }
        }
        self.facts
    }
}

/// Associated type value assigned by a trait implementation.
///
/// For example, `impl Iterator for Vec { type Item = u32; }` lowers to one fact whose
/// `impl_self_tid` is `Vec`, `trait_tid` is `Iterator`, `assoc_type` is the definition of
/// `Iterator::Item`, and `value_tid` is `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct AssocTypeImplFact {
    /// Implementing self type in `impl Trait for Self`.
    pub(crate) impl_self_tid: TypeId,
    /// Implemented trait type in `impl Trait for Self`.
    pub(crate) trait_tid: TypeId,
    /// Associated type definition assigned by the impl item.
    pub(crate) assoc_type: DefId,
    /// Type assigned by the impl item.
    pub(crate) value_tid: TypeId,
}
