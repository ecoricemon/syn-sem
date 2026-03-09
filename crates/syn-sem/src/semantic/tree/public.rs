use super::{
    format::{self, Brief, PrintFilter},
    inner::PathTree,
    item::{ItemIndex, PrivItem, PubItem},
    node::{Node, NodeIndex},
    private::PrivPathTree,
    ty::{OwnedType, Type, TypeId, UniqueTypes},
    PathId,
};
use crate::{etc::util::IntoPathSegments, GetOwned};
use std::fmt;

/// Hides [`PrivPathTree`] from clients.
pub trait AsPrivPathTree<'gcx> {
    fn as_private_path_tree(&self) -> &PrivPathTree<'gcx>;
}

impl<'gcx> AsPrivPathTree<'gcx> for PubPathTree<'gcx> {
    fn as_private_path_tree(&self) -> &PrivPathTree<'gcx> {
        &self.inner
    }
}

impl<'gcx> AsPrivPathTree<'gcx> for PrivPathTree<'gcx> {
    fn as_private_path_tree(&self) -> &PrivPathTree<'gcx> {
        self
    }
}

pub struct PubPathTree<'gcx> {
    pub(crate) inner: PrivPathTree<'gcx>,
}

impl<'gcx> PubPathTree<'gcx> {
    pub const ROOT: NodeIndex = PathTree::<PrivItem>::ROOT;

    pub(crate) fn new(tree: PrivPathTree<'gcx>) -> Self {
        Self { inner: tree }
    }

    pub fn crate_node(&self) -> NodeIndex {
        self.inner.crate_node
    }

    pub fn crate_name(&self) -> String {
        self.inner.get_name_path(self.crate_node())
    }

    pub fn types(&self) -> &UniqueTypes<'gcx> {
        self.inner.types()
    }

    pub fn get_type(&self, tid: TypeId) -> &Type<'gcx> {
        self.inner.get_type(tid)
    }

    pub fn contains_generic_param_in_type(&self, tid: TypeId) -> bool {
        self.inner.contains_generic_param_in_type(tid)
    }

    /// Finds the node from the given `base` + `key` ignoring visibility.
    pub fn search<I>(&self, base: NodeIndex, key: I) -> Option<NodeIndex>
    where
        I: IntoPathSegments,
    {
        self.inner.norm_search(base, key)
    }

    /// Finds the visible items from the given `base` + `key`.
    pub fn traverse<I, F, R>(&self, base: NodeIndex, key: I, mut f: F) -> Option<R>
    where
        I: IntoPathSegments,
        F: FnMut(PathId, PubItem<'_>) -> Option<R>,
    {
        self.inner.traverse(base, key, |_vis, pid, item| {
            let item = PubItem::new(item)?;
            f(pid, item)
        })
    }

    pub fn parent_item<F>(&self, index: NodeIndex, mut filter: F) -> PathId
    where
        F: FnMut(PubItem<'_>) -> bool,
    {
        self.inner.parent_item(index, |item| {
            if let Some(item) = PubItem::new(item) {
                filter(item)
            } else {
                false
            }
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = (PathId, PubItem<'_>)> {
        self.inner
            .iter()
            .filter_map(|(pid, item)| PubItem::new(item).map(|item| (pid, item)))
    }

    pub fn get_name_path(&self, index: NodeIndex) -> String {
        self.inner.get_name_path(index)
    }

    pub fn node(&self, index: NodeIndex) -> PubNode<'_> {
        PubNode {
            inner: &self.inner[index],
        }
    }

    pub fn item(&self, pid: PathId) -> PubItem<'_> {
        PubItem::new(&self.inner[pid]).unwrap()
    }

    pub fn debug_brief(&self) -> Brief<'_, Self> {
        Brief::new(self)
    }
}

impl GetOwned<TypeId> for PubPathTree<'_> {
    type Owned = OwnedType;

    fn get_owned(&self, tid: TypeId) -> Self::Owned {
        self.inner.get_owned(tid)
    }
}

impl Default for PubPathTree<'_> {
    fn default() -> Self {
        todo!()
    }
}

impl fmt::Debug for PubPathTree<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl format::DebugBriefly for PubPathTree<'_> {
    fn fmt_briefly(&self, f: &mut fmt::Formatter<'_>, filter: &PrintFilter) -> fmt::Result {
        self.inner.fmt_briefly(f, filter)
    }

    fn name(&self) -> &'static str {
        "PubPathTree"
    }
}

pub struct PubNode<'a> {
    inner: &'a Node<PrivItem>,
}

impl<'a> PubNode<'a> {
    pub fn iter(&self) -> impl Iterator<Item = (ItemIndex, PubItem<'_>)> + Clone {
        self.inner
            .iter()
            .filter_map(|(index, item)| PubItem::new(item).map(|item| (index, item)))
    }
}

pub mod pub_filter {
    use crate::semantic::tree::{inner::filter, public::PubItem};

    pub fn block(item: PubItem<'_>) -> bool {
        filter::block(&item)
    }

    pub fn block_fn(item: PubItem<'_>) -> bool {
        filter::block_fn(&item)
    }

    pub fn block_mod(item: PubItem<'_>) -> bool {
        filter::block_mod(&item)
    }

    pub fn block_mod_struct(item: PubItem<'_>) -> bool {
        filter::block_mod_struct(&item)
    }

    pub fn enum_(item: PubItem<'_>) -> bool {
        filter::enum_(&item)
    }

    pub fn mod_(item: PubItem<'_>) -> bool {
        filter::mod_(&item)
    }

    pub fn struct_(item: PubItem<'_>) -> bool {
        filter::struct_(&item)
    }
}
