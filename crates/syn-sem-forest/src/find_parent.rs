use crate::{
    identify::{IdentifySyn, SynId},
    Map,
};
use std::{any::TypeId, iter, slice};
use syn::punctuated;

#[derive(Debug, Clone, Default)]
pub struct ParentFinder {
    /// Mapping child -> parent.
    map: Map<SynId, SynId>,
}

impl ParentFinder {
    pub fn insert(&mut self, child: SynId, parent: SynId) {
        let _old_parent = self.map.insert(child, parent);

        #[cfg(debug_assertions)]
        if let Some(old_parent) = _old_parent {
            panic!(
                "conflict parent-child syn id: child: {}, old parent: {}, new parent: {}",
                child.content(),
                old_parent.content(),
                parent.content()
            );
        }
    }

    pub fn get_parent(&self, child: SynId) -> Option<&SynId> {
        self.map.get(&child)
    }

    /// Finds the nearest ancestor that is one type of the given types in the syntax tree.
    ///
    /// If found, returns its index to the `target_ancestors` and its syn id.
    pub fn get_ancestor(
        &self,
        child: SynId,
        target_ancestors: &[TypeId],
    ) -> Option<(usize, SynId)> {
        let mut cur = child;
        while let Some(parent) = self.get_parent(cur) {
            if let Some((index, _)) = target_ancestors
                .iter()
                .enumerate()
                .find(|(_, target)| **target == parent.as_any().type_id())
            {
                return Some((index, *parent));
            }
            cur = *parent;
        }
        None
    }
}

pub trait InsertRelation {
    /// Inserts parent-child relations to the given `finder`.
    ///
    /// Implementers are encouraged to call the same method on children as well so that clients
    /// can get the whole relationship by just one function call.
    fn insert_relation(&self, finder: &mut ParentFinder);
}

impl<T: InsertRelation> InsertRelation for Option<T> {
    fn insert_relation(&self, finder: &mut ParentFinder) {
        if let Some(this) = self {
            this.insert_relation(finder);
        }
    }
}

/// A helper trait for easy implementation of the [`InsertRelation`].
///
/// Lots of nodes in [`syn`]'s syntax tree wrapped in `Box`, `Option`, and others. This trait
/// unwraps those shells so that you can ignore their existence.
pub trait AsElements {
    type Output<'a>: Iterator<Item = Node>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_>;
}

impl<T: AsElements> AsElements for Option<T> {
    type Output<'a>
        = Elements<T::Output<'a>>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        if let Some(v) = self {
            Elements::Iter(v.as_elements())
        } else {
            Elements::Empty
        }
    }
}

impl<T: AsElements> AsElements for Box<T> {
    type Output<'a>
        = T::Output<'a>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        (**self).as_elements()
    }
}

impl<T: AsElements> AsElements for Vec<T> {
    type Output<'a>
        = Flatten<'a, slice::Iter<'a, T>, T>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        Flatten {
            iters: self.iter(),
            nodes: None,
        }
    }
}

impl<T: AsElements, P> AsElements for syn::punctuated::Punctuated<T, P> {
    type Output<'a>
        = Flatten<'a, punctuated::Iter<'a, T>, T>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        Flatten {
            iters: self.iter(),
            nodes: None,
        }
    }
}

impl<T0, T1> AsElements for (T0, T1)
where
    T0: AsElements,
    T1: AsElements,
{
    type Output<'a>
        = iter::Chain<T0::Output<'a>, T1::Output<'a>>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        self.0.as_elements().chain(self.1.as_elements())
    }
}

impl<T0, T1, T2> AsElements for (T0, T1, T2)
where
    T0: AsElements,
    T1: AsElements,
    T2: AsElements,
{
    type Output<'a>
        = iter::Chain<iter::Chain<T0::Output<'a>, T1::Output<'a>>, T2::Output<'a>>
    where
        Self: 'a;

    fn as_elements(&self) -> Self::Output<'_> {
        self.0
            .as_elements()
            .chain(self.1.as_elements())
            .chain(self.2.as_elements())
    }
}

pub enum Elements<I> {
    Iter(I),
    Empty,
}

impl<I: Iterator<Item = Node>> Iterator for Elements<I> {
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Iter(iter) => iter.next(),
            Self::Empty => None,
        }
    }
}

pub struct Flatten<'a, I, T: AsElements + 'a> {
    iters: I,
    nodes: Option<T::Output<'a>>,
}

impl<'a, I, T> Iterator for Flatten<'a, I, T>
where
    I: Iterator<Item = &'a T>,
    T: AsElements + 'a,
{
    type Item = Node;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(nodes) = self.nodes.as_mut() {
            let node = nodes.next();
            if node.is_some() {
                return node;
            }
        }

        for next in self.iters.by_ref() {
            let mut nodes = next.as_elements();
            let node = nodes.next();
            if node.is_some() {
                self.nodes = Some(nodes);
                return node;
            }
        }

        None
    }
}

#[derive(Clone, Copy)]
pub struct Node {
    sid: SynId,
    ptr_rel: *const dyn InsertRelation,
}

impl Node {
    #[inline]
    pub fn from<T: IdentifySyn + InsertRelation>(t: &T) -> Self {
        Self {
            sid: t.syn_id(),
            ptr_rel: t as *const T as *const dyn InsertRelation,
        }
    }

    pub const fn syn_id(&self) -> SynId {
        self.sid
    }

    pub fn as_dyn_insert_relation(&self) -> &dyn InsertRelation {
        unsafe { self.ptr_rel.as_ref().unwrap() }
    }
}
