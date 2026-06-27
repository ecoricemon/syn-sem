//! Impl associated-type collection for inference.

use crate::{InferTypes, TypeId, TypeLowerer};
use syn_sem_hir as hir;
use syn_sem_name::{DefId, NameDb, Namespace, ResolveResult};

pub(crate) struct ImplAssocTypeCollector<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    names: &'a NameDb<'cx>,
    ty_lowerer: TypeLowerer<'a, 'cx>,
    impl_assoc_types: Vec<ImplAssocType>,
}

impl<'a, 'cx> ImplAssocTypeCollector<'a, 'cx> {
    pub(crate) fn collect(
        hir: &'a hir::Hir<'cx>,
        names: &'a NameDb<'cx>,
        types: &'a mut InferTypes<'cx>,
    ) -> Vec<ImplAssocType> {
        Self {
            hir,
            names,
            ty_lowerer: TypeLowerer::new(hir, names, types),
            impl_assoc_types: Vec::new(),
        }
        .collect_inner()
    }

    fn collect_inner(mut self) -> Vec<ImplAssocType> {
        for item in self.hir.items() {
            let hir::ItemKind::Impl {
                trait_,
                self_,
                items,
                ..
            } = &item.kind
            else {
                continue;
            };
            let Some(trait_) = trait_ else {
                continue;
            };
            let impl_self = self.ty_lowerer.lower_hir_type(*self_);
            let trait_ty_id = self
                .ty_lowerer
                .lower_plain_path_as_type(trait_, item.parent_scope);
            let Some(trait_def) = self.ty_lowerer.trait_def_for_type(trait_ty_id) else {
                continue;
            };
            for assoc_item in items.iter().map(|id| &self.hir[*id]) {
                let hir::AssocItemKind::ImplType { ty } = assoc_item.kind else {
                    continue;
                };
                let ResolveResult::Found(assoc) =
                    self.names
                        .member(trait_def, Namespace::Type, assoc_item.name)
                else {
                    continue;
                };
                let value_ty = self.ty_lowerer.lower_hir_type(ty);
                self.impl_assoc_types.push(ImplAssocType {
                    impl_self,
                    trait_: trait_ty_id,
                    assoc,
                    value_ty,
                });
            }
        }
        self.impl_assoc_types
    }
}

/// Associated type value assigned by a trait implementation.
///
/// For example, `impl Iterator for Vec { type Item = u32; }` lowers to one fact whose `impl_self`
/// is `Vec`, `trait_` is `Iterator`, `assoc` is the definition of `Iterator::Item`, and `value_ty`
/// is `u32`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ImplAssocType {
    /// Implementing self type in `impl Trait for Self`.
    pub(crate) impl_self: TypeId,
    /// Implemented trait type in `impl Trait for Self`.
    pub(crate) trait_: TypeId,
    /// Associated type definition assigned by the impl item.
    pub(crate) assoc: DefId,
    /// Type assigned by the impl item.
    pub(crate) value_ty: TypeId,
}
