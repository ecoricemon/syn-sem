//! # Visibility
//!
//! Some [`Item`]s contain `vis_node` fields. The field indicates a node
//! index to a module that the item is visible. Look at an example below.
//! ```ignore
//! mod a {
//!     mod b {
//!         struct A;            // vis_node: Node index to 'mod b'
//!         pub(super) struct B; // vis_node: Node index to 'mod a'
//!     }
//! }
//! ```
//! We use node index instead of path id becuase a node can have only one
//! module, plus, it makes visibility check easy.

// TOOD: Delete unnecessary functions.

use super::{
    format,
    format::PrintFilter,
    inner,
    item::{self, PrivItem},
    node::{Node, NodeIndex},
    PathId,
};
use crate::etc::util::IntoPathSegments;
use hashlink::LinkedHashSet;
use std::{fmt, ops};

pub struct PrivPathTree<'gcx> {
    pub(crate) inner: inner::PathTree<'gcx, PrivItem>,
    pub(crate) unresolved: LinkedHashSet<PathId>,
}

impl<'gcx> PrivPathTree<'gcx> {
    pub(crate) fn new() -> Self {
        Self {
            inner: inner::PathTree::new(PrivItem::Mod),
            unresolved: LinkedHashSet::default(),
        }
    }

    pub(crate) fn crate_node(&self) -> NodeIndex {
        self.crate_node
    }

    pub(crate) fn num_nodes(&self) -> usize {
        self.inner.nodes.len()
    }

    pub(crate) fn add_item<I>(&mut self, base: NodeIndex, key: I, item: PrivItem) -> PathId
    where
        I: IntoPathSegments,
    {
        let is_raw = item.is_raw();
        match self.inner.try_add_item(base, key, item) {
            Ok(new_pid) => {
                if is_raw {
                    self.unresolved.insert(new_pid);
                }
                new_pid
            }
            Err((exist_pid, _)) => exist_pid,
        }
    }

    pub(crate) fn unresolved(&self) -> impl ExactSizeIterator<Item = PathId> + Clone + '_ {
        self.unresolved.iter().cloned()
    }

    pub(crate) fn set_item(&mut self, pid: PathId, item: PrivItem) {
        if item.is_raw() {
            debug_assert!(self.inner[pid].is_raw());
        } else {
            self.unresolved.remove(&pid);
        }
        self.inner[pid] = item;
    }

    pub(crate) fn get_mut_item(&mut self, pid: PathId) -> ItemMut<'_, 'gcx> {
        ItemMut { tree: self, pid }
    }

    pub(crate) fn take_item(&mut self, pid: PathId) -> PrivItem {
        self.unresolved.remove(&pid);
        self.inner.take_item(pid)
    }
}

impl Default for PrivPathTree<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for PrivPathTree<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl format::DebugBriefly for PrivPathTree<'_> {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result {
        self.inner.fmt_briefly(f, filter)
    }

    fn name(&self) -> &'static str {
        "PrivPathTree"
    }
}

impl<'gcx> ops::Deref for PrivPathTree<'gcx> {
    type Target = inner::PathTree<'gcx, PrivItem>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl ops::DerefMut for PrivPathTree<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl ops::Index<PathId> for PrivPathTree<'_> {
    type Output = PrivItem;

    fn index(&self, index: PathId) -> &Self::Output {
        &self.inner[index]
    }
}

impl ops::Index<NodeIndex> for PrivPathTree<'_> {
    type Output = Node<PrivItem>;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.inner[index]
    }
}

/// This type only supports methods that cannot change the whole items. In other words, you can only
/// modify some fields of items, not the items themselves. Use [`PrivPathTree::set_item`] to change
/// the whole item.
pub(crate) struct ItemMut<'a, 'gcx> {
    tree: &'a mut PrivPathTree<'gcx>,
    pid: PathId,
}

impl ItemMut<'_, '_> {
    pub(crate) fn as_mod(&mut self) -> &mut item::Mod {
        self.tree.inner[self.pid].as_mut_mod()
    }

    pub(crate) fn as_type_alias(&mut self) -> &mut item::TypeAlias {
        self.tree.inner[self.pid].as_mut_type_alias()
    }

    pub(crate) fn as_use(&mut self) -> &mut item::Use {
        self.tree.inner[self.pid].as_mut_use()
    }

    pub(crate) fn as_raw_const(&mut self) -> &mut item::RawConst {
        self.tree.inner[self.pid].as_mut_raw_const()
    }

    pub(crate) fn as_raw_enum(&mut self) -> &mut item::RawEnum {
        self.tree.inner[self.pid].as_mut_raw_enum()
    }

    pub(crate) fn as_raw_field(&mut self) -> &mut item::RawField {
        self.tree.inner[self.pid].as_mut_raw_field()
    }

    pub(crate) fn as_raw_fn(&mut self) -> &mut item::RawFn {
        self.tree.inner[self.pid].as_mut_raw_fn()
    }

    pub(crate) fn as_raw_mod(&mut self) -> &mut item::RawMod {
        self.tree.inner[self.pid].as_mut_raw_mod()
    }

    pub(crate) fn as_raw_struct(&mut self) -> &mut item::RawStruct {
        self.tree.inner[self.pid].as_mut_raw_struct()
    }

    pub(crate) fn as_raw_type_alias(&mut self) -> &mut item::RawTypeAlias {
        self.tree.inner[self.pid].as_mut_raw_type_alias()
    }

    pub(crate) fn as_raw_use(&mut self) -> &mut item::RawUse {
        self.tree.inner[self.pid].as_mut_raw_use()
    }

    pub(crate) fn as_raw_variant(&mut self) -> &mut item::RawVariant {
        self.tree.inner[self.pid].as_mut_raw_variant()
    }
}
