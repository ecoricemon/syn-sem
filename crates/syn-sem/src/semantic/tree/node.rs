use super::{item::ItemIndex, PathId};
use smallvec::SmallVec;
use std::{cmp, fmt, iter, mem, ops};

#[derive(Debug, Clone)]
pub struct Node<T> {
    pub(crate) items: SmallVec<[T; 1]>,
    pub(crate) parent: NodeIndex,
    pub(crate) children: Vec<(String, NodeIndex)>, // TODO: String -> Interned
}

impl<T> Node<T> {
    pub fn iter(&self) -> NodeIter<'_, T> {
        NodeIter::new(&self.items)
    }

    pub fn iter_mut(&mut self) -> NodeIterMut<'_, T> {
        NodeIterMut::new(&mut self.items)
    }

    pub(crate) fn push(&mut self, item: T) -> ItemIndex {
        self.items.push(item);
        ItemIndex(self.items.len() - 1)
    }
}

impl<T: Default> Node<T> {
    /// # Panics
    ///
    /// Panics if the given index is out of bounds.
    pub(crate) fn take(&mut self, ii: ItemIndex) -> T {
        mem::take(&mut self.items[ii.0])
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeIndex(pub(crate) usize);

impl NodeIndex {
    pub fn to_path_id<I: Into<ItemIndex>>(self, ii: I) -> PathId {
        PathId {
            ni: self,
            ii: ii.into(),
        }
    }
}

impl From<usize> for NodeIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

impl fmt::Display for NodeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for NodeIndex {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl PartialEq<usize> for NodeIndex {
    fn eq(&self, other: &usize) -> bool {
        self.0.eq(other)
    }
}

impl PartialOrd<usize> for NodeIndex {
    fn partial_cmp(&self, other: &usize) -> Option<cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl ops::Add<usize> for NodeIndex {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs)
    }
}

impl ops::AddAssign<usize> for NodeIndex {
    fn add_assign(&mut self, rhs: usize) {
        self.0 += rhs;
    }
}

impl ops::Sub<usize> for NodeIndex {
    type Output = Self;

    fn sub(self, rhs: usize) -> Self::Output {
        Self(self.0 - rhs)
    }
}

impl<T> ops::Index<NodeIndex> for [Node<T>] {
    type Output = Node<T>;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self[index.0]
    }
}

impl<T> ops::IndexMut<NodeIndex> for [Node<T>] {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self[index.0]
    }
}

impl<T> ops::Index<NodeIndex> for Vec<Node<T>> {
    type Output = Node<T>;

    fn index(&self, index: NodeIndex) -> &Self::Output {
        &self.as_slice()[index]
    }
}

impl<T> ops::IndexMut<NodeIndex> for Vec<Node<T>> {
    fn index_mut(&mut self, index: NodeIndex) -> &mut Self::Output {
        &mut self.as_mut_slice()[index]
    }
}

/// An iterator traversing a node's items and yielding pairs of item index and reference to an
/// [`Item`].
pub struct NodeIter<'a, T> {
    items: &'a [T],
    ii: usize,
}

impl<'a, T> NodeIter<'a, T> {
    pub(crate) const fn new(items: &'a [T]) -> Self {
        Self { items, ii: 0 }
    }

    pub fn empty() -> Self {
        Self { items: &[], ii: 0 }
    }
}

impl<'a, T> Clone for NodeIter<'a, T> {
    fn clone(&self) -> Self {
        Self {
            items: self.items,
            ii: self.ii,
        }
    }
}

impl<'a, T> Iterator for NodeIter<'a, T> {
    type Item = (ItemIndex, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(item) = self.items.get(self.ii) {
            let item = (ItemIndex(self.ii), item);
            self.ii += 1;
            Some(item)
        } else {
            None
        }
    }
}

impl<T> iter::FusedIterator for NodeIter<'_, T> {}

/// An iterator traversing a node's items and yielding pairs of item index and mutable reference to
/// an [`Item`].
pub struct NodeIterMut<'a, T> {
    items: &'a mut [T],
    ii: usize,
}

impl<'a, T> NodeIterMut<'a, T> {
    pub(crate) fn new(items: &'a mut [T]) -> Self {
        Self { items, ii: 0 }
    }

    pub fn empty() -> Self {
        Self {
            items: &mut [],
            ii: 0,
        }
    }
}

impl<'a, T> Iterator for NodeIterMut<'a, T> {
    type Item = (ItemIndex, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        match mem::take(&mut self.items) {
            [] => None,
            [head, tail @ ..] => {
                let item = (ItemIndex(self.ii), head);

                self.items = tail;
                self.ii += 1;

                Some(item)
            }
        }
    }
}

impl<T> iter::FusedIterator for NodeIterMut<'_, T> {}
