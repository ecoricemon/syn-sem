pub(crate) mod common;
pub(crate) mod file;

use crate::Map;
use common::{InsertRelation, ParentFinder, SynId};
use file::File;
use std::{
    any::{Any, TypeId},
    borrow::Borrow,
    hash::Hash,
    path::PathBuf,
    pin::Pin,
};

#[derive(Debug, Clone)]
pub struct SyntaxTree {
    /// Mapping between a file path and AST of a file.
    ///
    /// Syntax tree will never change after it is constructed.
    files: Map<PathBuf, Pin<Box<File>>>,

    /// Mapping between a file path and AST of an impl block.
    ///
    /// Impl blocks can be cloned and registered for monomorphization.
    impls: Map<PathBuf, Pin<Box<syn::ItemImpl>>>,

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

    pub(crate) fn insert_impl(&mut self, path: PathBuf, impl_: Pin<Box<syn::ItemImpl>>) {
        impl_.insert_relation(&mut self.parent_finder);
        self.impls.insert(path, impl_);
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
