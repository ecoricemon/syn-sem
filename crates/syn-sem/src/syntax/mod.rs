pub(crate) mod common;
pub(crate) mod file;

use crate::Map;
use common::{InsertRelation, ParentFinder, SynId};
use file::File;
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    fmt,
    hash::Hash,
    marker::PhantomPinned,
    path::PathBuf,
    pin::Pin,
};
use syn_locator::{Locate, LocateEntry, Location, Locator};

pub(crate) struct ClonedImpl {
    pub(crate) item_impl: syn::ItemImpl,
    pub(crate) locator: Locator,
    _pin: PhantomPinned,
}

impl ClonedImpl {
    pub(crate) fn new(
        item_impl: syn::ItemImpl,
        file_path: &str,
        code: String,
    ) -> crate::Result<Pin<Box<Self>>> {
        let mut this = Box::pin(Self {
            item_impl,
            locator: Locator::new(file_path, code),
            _pin: PhantomPinned,
        });

        unsafe {
            let this = Pin::as_mut(&mut this).get_unchecked_mut();
            this.item_impl.locate_as_entry(&mut this.locator)?;
        }

        Ok(this)
    }
}

impl fmt::Debug for ClonedImpl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ClonedImpl")
            .field("item_impl", &self.item_impl)
            .field("file_path", &self.locator.file_path())
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
pub struct SyntaxTree {
    /// Mapping between a file path and AST of a file.
    ///
    /// Syntax tree will never change after it is constructed.
    files: Map<PathBuf, Pin<Box<File>>>,

    /// Mapping between a file path and AST of an impl block.
    ///
    /// Impl blocks can be cloned and registered for monomorphization.
    impls: Map<PathBuf, Pin<Box<ClonedImpl>>>,

    parent_finder: ParentFinder,
}

impl SyntaxTree {
    pub(crate) fn new() -> Self {
        Self {
            files: Map::default(),
            impls: Map::default(),
            parent_finder: ParentFinder::new(),
        }
    }

    pub fn files(&self) -> impl ExactSizeIterator<Item = &syn::File> + Clone {
        self.files.values().map(|file| &file.file)
    }

    pub(crate) fn contains_file<Q>(&self, path: &Q) -> bool
    where
        PathBuf: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.files.contains_key(path)
    }

    pub(crate) fn get_file<Q>(&self, path: &Q) -> Option<&File>
    where
        PathBuf: Borrow<Q>,
        Q: Hash + Eq + ?Sized,
    {
        self.files.get(path).map(|pinned| &**pinned)
    }

    pub(crate) fn insert_file(&mut self, path: PathBuf, file: Pin<Box<File>>) {
        file.insert_relation(&mut self.parent_finder);
        self.files.insert(path, file);
    }

    pub(crate) fn insert_impl(&mut self, path: PathBuf, impl_: Pin<Box<ClonedImpl>>) {
        impl_.item_impl.insert_relation(&mut self.parent_finder);
        self.impls.insert(path, impl_);
    }

    pub(crate) fn locator_of(&self, sid: SynId) -> Option<&Locator> {
        let mut cur = sid;
        loop {
            if let Some(locator) = self.locator_of_direct(cur) {
                return Some(locator);
            }
            cur = *self.get_parent(cur)?;
        }
    }

    pub(crate) fn location<T>(&self, node: &T) -> Location
    where
        T: common::IdentifySyn + Locate,
    {
        let locator = self
            .locator_of(node.syn_id())
            .expect("failed to find locator for syntax node");
        node.location(locator)
    }

    pub(crate) fn code<T>(&self, node: &T) -> String
    where
        T: common::IdentifySyn + Locate,
    {
        let locator = self
            .locator_of(node.syn_id())
            .expect("failed to find locator for syntax node");
        node.code(locator)
    }

    fn locator_of_direct(&self, sid: SynId) -> Option<&Locator> {
        if let Some(ptr) = sid.as_const_ptr::<File>() {
            return self
                .files
                .values()
                .find(|owned| std::ptr::eq(&***owned, ptr))
                .map(|owned| &owned.locator);
        }
        if let Some(ptr) = sid.as_const_ptr::<syn::File>() {
            return self
                .files
                .values()
                .find(|owned| std::ptr::eq(&owned.file, ptr))
                .map(|owned| &owned.locator);
        }
        if let Some(ptr) = sid.as_const_ptr::<syn::ItemImpl>() {
            return self
                .impls
                .values()
                .find(|owned| std::ptr::eq(&owned.item_impl, ptr))
                .map(|owned| &owned.locator);
        }
        None
    }

    pub(crate) fn get_parent(&self, child: SynId) -> Option<&SynId> {
        self.parent_finder.get_parent(child)
    }

    /// Finds the nearest ancestor that is one type of the given types in the syntax tree.
    ///
    /// If found, returns its index to the `target_ancestors` and its syn id.
    pub(crate) fn get_ancestor(
        &self,
        child: SynId,
        target_ancestors: &[TypeId],
    ) -> Option<(usize, SynId)> {
        self.parent_finder.get_ancestor(child, target_ancestors)
    }

    pub(crate) fn get_ancestor1<A>(&self, child: SynId) -> Option<&A>
    where
        A: Any,
    {
        let targets = [TypeId::of::<A>()];
        self.parent_finder
            .get_ancestor(child, &targets)
            .map(|(_index, sid)| Self::downcast(sid))
    }

    fn downcast<'o, T: Any>(sid: SynId) -> &'o T {
        unsafe {
            let ref_ = sid.as_any().downcast_ref::<T>().unwrap_unchecked();
            let ptr = ref_ as *const T;
            ptr.as_ref().unwrap_unchecked()
        }
    }
}

impl Default for SyntaxTree {
    fn default() -> Self {
        Self::new()
    }
}
