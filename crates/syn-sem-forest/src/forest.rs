use crate::{File, IdentifySyn, InsertRelation, ParentFinder, Result, SynId};
use std::{
    any::{Any, TypeId},
    fmt,
    marker::PhantomPinned,
    pin::Pin,
};
use syn_locator::{Locate, LocateEntry, Location, Locator};
use syn_sem_common::{FilePath, Map};

#[derive(Debug, Default)]
/// A collection of pinned raw `syn` syntax trees with lookup metadata.
pub struct SyntaxForest<'cx> {
    /// Mapping between a file path and syntax tree of a file.
    ///
    /// Syntax trees will never change after they are constructed.
    files: Map<FilePath<'cx>, Pin<Box<File<'cx>>>>,

    /// Mapping between a synthetic file path and syntax tree of an impl block.
    ///
    /// Impl blocks can be cloned and registered for monomorphization.
    impls: Map<FilePath<'cx>, Pin<Box<ClonedImpl>>>,

    parent_finder: ParentFinder,
}

impl<'cx> SyntaxForest<'cx> {
    /// Iterates parsed `syn::File` roots.
    pub fn files(&self) -> impl ExactSizeIterator<Item = &syn::File> + Clone {
        self.files.values().map(|file| &file.file)
    }

    /// Returns whether the forest contains `file_path`.
    pub fn contains_file(&self, file_path: FilePath<'cx>) -> bool {
        self.files.contains_key(&file_path)
    }

    /// Returns the parsed file stored for `file_path`.
    pub fn get_file(&self, file_path: FilePath<'cx>) -> Option<&File<'cx>> {
        self.files.get(&file_path).map(|pinned| &**pinned)
    }

    /// Inserts a parsed file and records its parent relationships.
    pub fn insert_file(&mut self, file_path: FilePath<'cx>, file: Pin<Box<File<'cx>>>) {
        file.file.insert_relation(&mut self.parent_finder);
        self.files.insert(file_path, file);
    }

    /// Inserts a cloned impl block and records its parent relationships.
    pub fn insert_impl(&mut self, file_path: FilePath<'cx>, impl_: Pin<Box<ClonedImpl>>) {
        impl_.item_impl.insert_relation(&mut self.parent_finder);
        self.impls.insert(file_path, impl_);
    }

    /// Returns the locator that owns the node identified by `sid`.
    pub fn locator_of(&self, sid: SynId) -> Option<&Locator> {
        let mut cur = sid;
        loop {
            if let Some(locator) = self.locator_of_direct(cur) {
                return Some(locator);
            }
            cur = *self.get_parent(cur)?;
        }
    }

    /// Returns the source location for `node`.
    pub fn location<T: IdentifySyn + Locate>(&self, node: &T) -> Location {
        let locator = self
            .locator_of(node.syn_id())
            .expect("failed to find locator for syntax node");
        node.location(locator)
    }

    /// Returns the source text for `node`.
    pub fn code<T: IdentifySyn + Locate>(&self, node: &T) -> String {
        let locator = self
            .locator_of(node.syn_id())
            .expect("failed to find locator for syntax node");
        node.code(locator)
    }

    fn locator_of_direct(&self, sid: SynId) -> Option<&Locator> {
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

    /// Returns the direct parent of `child`.
    pub fn get_parent(&self, child: SynId) -> Option<&SynId> {
        self.parent_finder.get_parent(child)
    }

    /// Finds the nearest ancestor whose type is in `target_ancestors`.
    ///
    /// If found, returns its index to the `target_ancestors` and its syn id.
    pub fn get_ancestor(
        &self,
        child: SynId,
        target_ancestors: &[TypeId],
    ) -> Option<(usize, SynId)> {
        self.parent_finder.get_ancestor(child, target_ancestors)
    }

    /// Finds the nearest ancestor of type `A`.
    pub fn get_ancestor1<A>(&self, child: SynId) -> Option<&A>
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

/// A cloned `syn::ItemImpl` with independent locator state.
pub struct ClonedImpl {
    /// Cloned impl item.
    pub item_impl: syn::ItemImpl,
    /// Locator populated for the cloned impl.
    pub locator: Locator,
    _pin: PhantomPinned,
}

impl ClonedImpl {
    /// Creates a pinned cloned impl from source text.
    pub fn new(item_impl: syn::ItemImpl, file_path: &str, code: String) -> Result<Pin<Box<Self>>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AttributeHelper, FindChild};
    use std::any::TypeId;
    use syn_locator::Find;
    use syn_sem_common::CommonCx;

    fn sample_forest<'cx>(ccx: &'cx CommonCx) -> (SyntaxForest<'cx>, FilePath<'cx>) {
        let file_path = ccx.intern("/virtual/main.rs");
        let file = File::new(
            file_path,
            r#"
                #[outer]
                mod a {
                    #[inner]
                    fn f() {
                        let x = 1usize;
                    }
                }
            "#,
        )
        .unwrap();

        let mut forest = SyntaxForest::default();
        forest.insert_file(file_path, file);
        (forest, file_path)
    }

    fn sample_mod(file: &syn::File) -> &syn::ItemMod {
        match &file.items[0] {
            syn::Item::Mod(item_mod) => item_mod,
            _ => panic!("expected sample module"),
        }
    }

    fn sample_fn(item_mod: &syn::ItemMod) -> &syn::ItemFn {
        let (_, items) = item_mod.content.as_ref().unwrap();
        match &items[0] {
            syn::Item::Fn(item_fn) => item_fn,
            _ => panic!("expected sample function"),
        }
    }

    fn sample_local(item_fn: &syn::ItemFn) -> &syn::Local {
        match &item_fn.block.stmts[0] {
            syn::Stmt::Local(local) => local,
            _ => panic!("expected sample local"),
        }
    }

    #[test]
    fn file_new_parses_and_records_locations() {
        let ccx = CommonCx::new();
        let file_path = ccx.intern("/virtual/basic.rs");
        let file = File::new(file_path, "fn main() {}").unwrap();

        assert_eq!(file.file_path, file_path);
        assert_eq!(file.file.items.len(), 1);
        assert_eq!(file.locator.file_path(), "/virtual/basic.rs");

        let item_fn =
            <syn::File as Find<syn::ItemFn>>::find(&file.file, &file.locator, "fn main() {}")
                .unwrap();
        assert_eq!(item_fn.code(&file.locator), "fn main() {}");
    }

    #[test]
    fn syntax_forest_insert_file_builds_parent_relationships() {
        let ccx = CommonCx::new();
        let (forest, file_path) = sample_forest(&ccx);
        let file = forest.get_file(file_path).unwrap();
        let item_mod = sample_mod(&file.file);
        let item_fn = sample_fn(item_mod);

        assert!(forest.contains_file(file_path));
        let mod_parent = forest.get_parent(item_mod.syn_id()).unwrap();
        assert!(mod_parent.as_ref::<syn::Item>().is_some());

        let fn_parent = forest.get_parent(item_fn.syn_id()).unwrap();
        assert!(fn_parent.as_ref::<syn::Item>().is_some());
        assert_eq!(
            forest
                .get_ancestor1::<syn::ItemMod>(item_fn.syn_id())
                .unwrap()
                .ident,
            "a"
        );
    }

    #[test]
    fn syntax_forest_returns_code_and_location_for_nodes() {
        let ccx = CommonCx::new();
        let (forest, file_path) = sample_forest(&ccx);
        let file = forest.get_file(file_path).unwrap();
        let item_fn = sample_fn(sample_mod(&file.file));

        assert!(forest.code(item_fn).contains("fn f()"));
        assert_eq!(forest.location(item_fn), item_fn.location(&file.locator));
        assert_eq!(
            forest.locator_of(item_fn.syn_id()).unwrap().file_path(),
            "/virtual/main.rs"
        );
    }

    #[test]
    fn ancestor_lookup_finds_nearest_requested_parent() {
        let ccx = CommonCx::new();
        let (forest, file_path) = sample_forest(&ccx);
        let file = forest.get_file(file_path).unwrap();
        let local = sample_local(sample_fn(sample_mod(&file.file)));

        let targets = [TypeId::of::<syn::ItemMod>(), TypeId::of::<syn::Block>()];
        let (index, ancestor) = forest.get_ancestor(local.syn_id(), &targets).unwrap();
        assert_eq!(index, 1);
        assert!(ancestor.as_ref::<syn::Block>().is_some());

        let block = forest.get_ancestor1::<syn::Block>(local.syn_id()).unwrap();
        assert_eq!(
            forest.code(block).trim(),
            "{\n                        let x = 1usize;\n                    }"
        );
    }

    #[test]
    fn find_child_finds_requested_descendants() {
        let file = syn::parse_str::<syn::File>(
            r#"
                fn f() {
                    let x = 1usize;
                    let y = x;
                }
            "#,
        )
        .unwrap();

        let targets = [TypeId::of::<syn::Local>()];
        let mut found = Vec::new();
        file.visit_descendant(&targets, &mut |index, sid| {
            found.push((index, sid.type_name()));
        });

        assert_eq!(found.len(), 2);
        assert!(found
            .iter()
            .all(|(index, name)| { *index == 0 && *name == std::any::type_name::<syn::Local>() }));
    }

    #[test]
    fn attribute_helper_reads_removes_and_replaces_attributes() {
        let mut item_struct = syn::parse_str::<syn::ItemStruct>(
            r#"
                #[derive(Debug)]
                #[repr(C)]
                struct S;
            "#,
        )
        .unwrap();

        assert!(item_struct.contains_attribute("derive"));
        assert!(item_struct.get_attribute_inner("derive").is_some());

        item_struct.remove_attribute("repr");
        assert!(!item_struct.contains_attribute("repr"));

        let new_attrs = vec![syn::parse_quote!(#[allow(dead_code)])];
        let old_attrs = item_struct.replace_attributes(new_attrs);
        assert_eq!(old_attrs.len(), 1);
        assert!(item_struct.contains_attribute("allow"));
    }

    #[test]
    fn cloned_impl_registers_with_locator_support() {
        let item_impl = syn::parse_str::<syn::ItemImpl>(
            r#"
                impl S {
                    fn f(&self) {}
                }
            "#,
        )
        .unwrap();
        let cloned = ClonedImpl::new(
            item_impl,
            "/virtual/impl.rs:1",
            "impl S {\n    fn f(&self) {}\n}".to_owned(),
        )
        .unwrap();
        let sid = cloned.item_impl.syn_id();

        let ccx = CommonCx::new();
        let file_path = ccx.intern("/virtual/impl.rs:1");
        let mut forest = SyntaxForest::default();
        forest.insert_impl(file_path, cloned);

        assert_eq!(
            forest.locator_of(sid).unwrap().file_path(),
            "/virtual/impl.rs:1"
        );
        let item_impl = sid.as_ref::<syn::ItemImpl>().unwrap();
        assert!(forest.code(item_impl).starts_with("impl S"));
    }
}
