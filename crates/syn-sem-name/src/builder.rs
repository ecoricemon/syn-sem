//! AST-aware collection into [`NameDb`].
//!
//! Callers provide already parsed files, so this module does not read source files or depend on a
//! top-level orchestration context.

use crate::{
    AstNodeId, DefId, DefKind, ImportId, ImportKind, NameDb, Namespace, Origin, ScopeId, ScopeKind,
};
use std::{borrow::Borrow, collections::BTreeMap, io, path::PathBuf};
use syn_sem_ast::{
    Block, GenericParam, Generics, ImplItem, Item, ItemConst, ItemEnum, ItemFn, ItemImpl, ItemMod,
    ItemStruct, ItemTrait, ItemType, ItemUse, ModulePath, Pat, Path, Signature, SourceInput, Stmt,
    TraitItem, UseTree, Visibility,
};
use syn_sem_common::{MaybeResult, Result, Set, Str};

/// Collects prepared AST inputs into a name database under construction.
pub(crate) struct AstCollector<'a, 'cx> {
    pub(crate) files: BTreeMap<Str<'cx>, SourceInput<'cx>>,
    pub(crate) db: &'a mut NameDb<'cx>,
}

impl<'a, 'cx> AstCollector<'a, 'cx> {
    pub(crate) fn collect_roots(
        &mut self,
        roots: impl IntoIterator<Item = Str<'cx>>,
    ) -> Result<()> {
        let crate_scope = NameDb::CRATE_SCOPE;
        let mut seen = Set::default();
        for root_path in roots {
            if !seen.insert(root_path) {
                continue;
            }
            let input = self.file(&root_path)?;
            let path = ModulePath::from_entry_file(PathBuf::from(root_path.as_ref()));
            for item in input.file.items {
                self.collect_item_from_module_tree(crate_scope, item, &path)?;
            }
        }
        Ok(())
    }

    fn file<Q>(&self, file_path: &Q) -> Result<SourceInput<'cx>>
    where
        Str<'cx>: Borrow<Q> + Ord,
        Q: Ord + AsRef<str> + ?Sized,
    {
        self.files.get(file_path).copied().ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "name collection input is missing root file: {}",
                    file_path.as_ref()
                ),
            )
            .into()
        })
    }

    fn child_file(
        &self,
        path: &ModulePath,
        module: &ItemMod<'cx>,
    ) -> MaybeResult<SourceInput<'cx>> {
        Ok(path
            .child_file_candidates(module)?
            .into_iter()
            .find_map(|candidate| self.files.get(candidate.to_str()?).copied()))
    }

    /// Collects one item while walking a crate's module tree.
    ///
    /// Unlike [`Self::collect_item_from_ast`], this variant follows `mod foo;` declarations to
    /// their source files and continues collecting there.
    fn collect_item_from_module_tree(
        &mut self,
        scope: ScopeId,
        item: &'cx Item<'cx>,
        path: &ModulePath,
    ) -> Result<()> {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            Item::Mod(item) => self.collect_mod_from_module_tree(scope, item, path, ast_node),
            _ => {
                self.collect_item_from_ast(scope, item);
                Ok(())
            }
        }
    }

    /// Collects one AST item into the current name database without loading extra files.
    ///
    /// Inline modules are collected recursively, declarations create `Def`s, and `use` trees are
    /// recorded as imports to be resolved after collection finishes.
    fn collect_item_from_ast(&mut self, scope: ScopeId, item: &'cx Item<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            Item::Const(item) => self.collect_const(scope, item, ast_node),
            Item::Enum(item) => self.collect_enum(scope, item, ast_node),
            Item::Fn(item) => self.collect_fn(scope, item, ast_node),
            Item::Impl(item) => self.collect_impl(scope, item, ast_node),
            Item::Mod(item) => self.collect_mod_from_ast(scope, item, ast_node),
            Item::Struct(item) => self.collect_struct(scope, item, ast_node),
            Item::Trait(item) => self.collect_trait(scope, item, ast_node),
            Item::Type(item) => self.collect_type(scope, item, ast_node),
            Item::Use(item) => self.collect_use(scope, item),
        }
    }

    fn collect_mod_from_module_tree(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemMod<'cx>,
        path: &ModulePath,
        ast_node: AstNodeId<'cx>,
    ) -> Result<()> {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def = self.add_named_def(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            visibility,
            ast_node,
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_path_scope(module_def, module_scope);

        if let Some(items) = item.items {
            let path = path.enter_inline_module(item)?;
            for item in items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        } else if let Some(input) = self.child_file(path, item)? {
            let path = path.enter_external_module(item, PathBuf::from(input.file_path.as_ref()))?;
            for item in input.file.items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        }

        Ok(())
    }

    fn collect_mod_from_ast(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemMod<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def = self.add_named_def(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            visibility,
            ast_node,
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_path_scope(module_def, module_scope);

        if let Some(items) = item.items {
            for item in items {
                self.collect_item_from_ast(module_scope, item);
            }
        }
    }

    /// For `const C: usize = 0;`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// └─ DefKind::Const C
    /// ```
    fn collect_const(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemConst<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Const
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        self.add_named_def(
            parent_scope,
            DefKind::Const,
            item.ident.inner,
            visibility,
            ast_node,
        );
    }

    /// For `enum E<T> { V }`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Enum E
    /// │  ├─ scopes.generic -> Generic scope
    /// │  └─ scopes.path    -> Item scope
    /// └─ Generic scope
    ///    └─ Item scope
    ///       └─ DefKind::Variant V
    /// ```
    ///
    /// Without generics, the hierarchy is still not flattened into `parent_scope`:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Enum E
    /// │  └─ scopes.path -> Item scope
    /// └─ Item scope
    ///    └─ DefKind::Variant V
    /// ```
    fn collect_enum(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemEnum<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Enum
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let enum_def = self.add_named_def(
            parent_scope,
            DefKind::Enum,
            item.ident.inner,
            visibility,
            ast_node,
        );

        // Generic scope
        let generic_scope = self.create_generic_scope(parent_scope, &item.generics);
        if let Some(generic_scope) = generic_scope {
            self.db.set_generic_scope(enum_def, generic_scope);
        }
        let path_parent_scope = generic_scope.unwrap_or(parent_scope);

        // DefKind::Variant
        let path_scope = self.db.add_scope(ScopeKind::Item, Some(path_parent_scope));
        self.db.set_path_scope(enum_def, path_scope);
        for variant in item.variants {
            self.add_named_def(
                path_scope,
                DefKind::Variant,
                variant.ident.inner,
                visibility,
                AstNodeId::from_ref(variant),
            );
        }
    }

    /// For `fn f<T>(x: T) { let y = x; }`, this creates the following definition and scope
    /// hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Fn f
    /// │  ├─ scopes.generic -> Generic scope
    /// │  └─ scopes.body    -> Function scope
    /// └─ Generic scope
    ///    └─ Function scope
    ///       ├─ DefKind::Local x
    ///       └─ Block scope
    ///          └─ DefKind::Local y
    /// ```
    ///
    /// Without generics, the function scope is attached directly under `parent_scope`:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Fn f
    /// │  └─ scopes.body -> Function scope
    /// └─ Function scope
    ///    ├─ DefKind::Local x
    ///    └─ Block scope
    ///       └─ DefKind::Local y
    /// ```
    fn collect_fn(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemFn<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Fn
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let fn_def = self.add_named_def(
            parent_scope,
            DefKind::Fn,
            item.sig.ident.inner,
            visibility,
            ast_node,
        );

        // Generic scope
        let generic_scope = self.create_generic_scope(parent_scope, &item.sig.generics);
        if let Some(generic_scope) = generic_scope {
            self.db.set_generic_scope(fn_def, generic_scope);
        }
        let function_parent_scope = generic_scope.unwrap_or(parent_scope);

        // Block scope
        let function_scope =
            self.create_function_body_scope(function_parent_scope, &item.sig, &item.block);
        self.db.set_body_scope(fn_def, function_scope);
    }

    /// For `struct S<T>;`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Struct S
    /// │  └─ scopes.generic -> Generic scope
    /// └─ Generic scope
    /// ```
    ///
    /// Without generics:
    ///
    /// ```text
    /// parent_scope
    /// └─ DefKind::Struct S
    /// ```
    fn collect_struct(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemStruct<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Struct
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(
            parent_scope,
            DefKind::Struct,
            item.ident.inner,
            visibility,
            ast_node,
        );

        // Generic scope
        if let Some(generic_scope) = self.create_generic_scope(parent_scope, &item.generics) {
            self.db.set_generic_scope(def, generic_scope);
        }
    }

    /// For `trait Tr<T> { ... }`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Trait Tr
    /// │  └─ scopes.generic -> Generic scope
    /// └─ Generic scope
    ///    └─ Trait scope
    /// ```
    ///
    /// Without trait generics:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Trait Tr
    /// └─ Trait scope
    /// ```
    ///
    /// Trait items inside the `Trait` scope are collected by [`Self::collect_trait_item`].
    fn collect_trait(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemTrait<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // Trait Def
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let trait_def = self.add_named_def(
            parent_scope,
            DefKind::Trait,
            item.ident.inner,
            visibility,
            ast_node,
        );

        // Generic scope
        let generic_scope = self.create_generic_scope(parent_scope, &item.generics);
        if let Some(generic_scope) = generic_scope {
            self.db.set_generic_scope(trait_def, generic_scope);
        }
        let trait_parent_scope = generic_scope.unwrap_or(parent_scope);

        // Trait scope
        let trait_scope = self
            .db
            .add_scope(ScopeKind::Trait, Some(trait_parent_scope));
        self.db.set_path_scope(trait_def, trait_scope);

        // Assoc items
        for item in item.items {
            self.collect_trait_item(trait_scope, item);
        }
    }

    /// For trait items `const C`, `type Assoc<T>`, and `fn f<U>(&self) {}`, this creates the
    /// following definition and scope hierarchy:
    ///
    /// ```text
    /// trait_scope
    /// ├─ DefKind::AssocConst C
    /// ├─ DefKind::AssocType Assoc
    /// │  └─ scopes.generic -> Generic scope
    /// ├─ Generic scope
    /// ├─ DefKind::AssocFn f
    /// │  ├─ scopes.generic -> Generic scope
    /// │  └─ scopes.body    -> Function scope
    /// └─ Generic scope
    ///    └─ Function scope
    ///       └─ Block scope
    /// ```
    fn collect_trait_item(&mut self, trait_scope: ScopeId, item: &'cx TraitItem<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            TraitItem::Const(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    self.nearest_module_scope(trait_scope),
                    ast_node,
                );
                if let Some(generic_scope) = self.create_generic_scope(trait_scope, &item.generics)
                {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
            TraitItem::Fn(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocFn,
                    item.sig.ident.inner,
                    self.nearest_module_scope(trait_scope),
                    ast_node,
                );
                let generic_scope = self.create_generic_scope(trait_scope, &item.sig.generics);
                if let Some(generic_scope) = generic_scope {
                    self.db.set_generic_scope(def, generic_scope);
                }
                let function_parent_scope = generic_scope.unwrap_or(trait_scope);
                if let Some(block) = &item.default {
                    let function_scope =
                        self.create_function_body_scope(function_parent_scope, &item.sig, block);
                    self.db.set_body_scope(def, function_scope);
                }
            }
            TraitItem::Type(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocType,
                    item.ident.inner,
                    self.nearest_module_scope(trait_scope),
                    ast_node,
                );
                if let Some(generic_scope) = self.create_generic_scope(trait_scope, &item.generics)
                {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
        }
    }

    /// For `impl<T> S<T> { ... }`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Impl
    /// │  └─ scopes.generic -> Generic scope
    /// └─ Generic scope
    ///    └─ Impl scope
    /// ```
    ///
    /// Without impl generics:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::Impl
    /// └─ Impl scope
    /// ```
    ///
    /// Impl items inside the `Impl` scope are collected by [`Self::collect_impl_item`].
    fn collect_impl(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemImpl<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Impl
        let impl_def = self.db.add_def(
            parent_scope,
            DefKind::Impl,
            None,
            self.nearest_module_scope(parent_scope),
            Origin::Ast(ast_node),
        );

        // Generic scope
        let generic_scope = self.create_generic_scope(parent_scope, &item.generics);
        if let Some(generic_scope) = generic_scope {
            self.db.set_generic_scope(impl_def, generic_scope);
        }
        let impl_parent_scope = generic_scope.unwrap_or(parent_scope);

        // Impl scope
        let impl_scope = self.db.add_scope(ScopeKind::Impl, Some(impl_parent_scope));

        // Assoc items
        for item in item.items {
            self.collect_impl_item(impl_scope, item);
        }
    }

    /// For impl items `const C`, `type Assoc<T>`, and `fn f<U>(&self) {}`, this creates the
    /// following definition and scope hierarchy:
    ///
    /// ```text
    /// impl_scope
    /// ├─ DefKind::AssocConst C
    /// ├─ DefKind::AssocType Assoc
    /// │  └─ scopes.generic -> Generic scope
    /// ├─ Generic scope
    /// ├─ DefKind::AssocFn f
    /// │  ├─ scopes.generic -> Generic scope
    /// │  └─ scopes.body    -> Function scope
    /// └─ Generic scope
    ///    └─ Function scope
    ///       └─ Block scope
    /// ```
    fn collect_impl_item(&mut self, impl_scope: ScopeId, item: &'cx ImplItem<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            ImplItem::Const(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    self.nearest_module_scope(impl_scope),
                    ast_node,
                );
                if let Some(generic_scope) = self.create_generic_scope(impl_scope, &item.generics) {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
            ImplItem::Fn(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocFn,
                    item.sig.ident.inner,
                    self.nearest_module_scope(impl_scope),
                    ast_node,
                );
                let generic_scope = self.create_generic_scope(impl_scope, &item.sig.generics);
                if let Some(generic_scope) = generic_scope {
                    self.db.set_generic_scope(def, generic_scope);
                }
                let function_parent_scope = generic_scope.unwrap_or(impl_scope);
                let function_scope =
                    self.create_function_body_scope(function_parent_scope, &item.sig, &item.block);
                self.db.set_body_scope(def, function_scope);
            }
            ImplItem::Type(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocType,
                    item.ident.inner,
                    self.nearest_module_scope(impl_scope),
                    ast_node,
                );
                if let Some(generic_scope) = self.create_generic_scope(impl_scope, &item.generics) {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
        }
    }

    /// For `type Alias<T> = T;`, this creates the following definition and scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// ├─ DefKind::TypeAlias Alias
    /// │  └─ scopes.generic -> Generic scope
    /// └─ Generic scope
    /// ```
    ///
    /// Without generics:
    ///
    /// ```text
    /// parent_scope
    /// └─ DefKind::TypeAlias Alias
    /// ```
    fn collect_type(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ItemType<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::TypeAlias
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(
            parent_scope,
            DefKind::TypeAlias,
            item.ident.inner,
            visibility,
            ast_node,
        );

        // Generic scope
        if let Some(generic_scope) = self.create_generic_scope(parent_scope, &item.generics) {
            self.db.set_generic_scope(def, generic_scope);
        }
    }

    /// For `use crate::a::B;`, this records the following import before import resolution:
    ///
    /// ```text
    /// scope
    /// └─ Import { source_path: [crate, a, B], kind: Single, visibility }
    /// ```
    ///
    /// After import resolution, the receiving scope gets an alias definition:
    ///
    /// ```text
    /// scope
    /// └─ DefKind::Use B -> target DefKind::Struct B
    /// ```
    fn collect_use(&mut self, scope: ScopeId, item: &'cx ItemUse<'cx>) {
        let visibility = self.visibility_from_ast(scope, &item.vis);
        let start = self.db.import_count();
        self.collect_use_tree(
            scope,
            Vec::new(),
            item.leading_colon.is_some(),
            &item.tree,
            visibility,
        );
        let imports = (start..self.db.import_count()).map(ImportId::new).collect();
        self.db
            .set_imports_ast_node(AstNodeId::from_ref(item), imports);
    }

    /// For `use crate::a::{B, C as D, *};`, this flattens the use tree into import records:
    ///
    /// ```text
    /// scope
    /// ├─ Import { source_path: [crate, a, B], kind: Single, visibility }
    /// ├─ Import { source_path: [crate, a, C], kind: Rename(D), visibility }
    /// └─ Import { source_path: [crate, a],    kind: Glob, visibility }
    /// ```
    ///
    /// Nested `Path` nodes extend `prefix`; `Group` nodes clone the current prefix for each branch.
    fn collect_use_tree(
        &mut self,
        scope: ScopeId,
        prefix: Vec<Str<'cx>>,
        is_absolute: bool,
        tree: &'cx UseTree<'cx>,
        visibility: ScopeId,
    ) {
        match tree {
            UseTree::Path(tree) => {
                let mut prefix = prefix;
                prefix.push(tree.ident.inner);
                self.collect_use_tree(scope, prefix, is_absolute, tree.tree, visibility);
            }
            UseTree::Name(tree) => {
                let mut source_path = prefix;
                source_path.push(tree.ident.inner);
                self.db.add_import(
                    scope,
                    source_path,
                    is_absolute,
                    ImportKind::Single,
                    visibility,
                    Origin::Untracked,
                );
            }
            UseTree::Rename(tree) => {
                let mut source_path = prefix;
                source_path.push(tree.ident.inner);
                self.db.add_import(
                    scope,
                    source_path,
                    is_absolute,
                    ImportKind::Rename(tree.rename.inner),
                    visibility,
                    Origin::Untracked,
                );
            }
            UseTree::Glob(_) => {
                self.db.add_import(
                    scope,
                    prefix,
                    is_absolute,
                    ImportKind::Glob,
                    visibility,
                    Origin::Untracked,
                );
            }
            UseTree::Group(tree) => {
                for tree in tree.items {
                    self.collect_use_tree(scope, prefix.clone(), is_absolute, tree, visibility);
                }
            }
        }
    }

    /// For `{ let x = 0; struct Local; x }`, this creates the following scope hierarchy:
    ///
    /// ```text
    /// parent_scope
    /// └─ Block scope
    ///    ├─ DefKind::Local x
    ///    └─ DefKind::Struct Local
    /// ```
    fn collect_block(&mut self, parent_scope: ScopeId, block: &'cx Block<'cx>) {
        // Block scope
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));
        self.db
            .set_scope_ast_node(block_scope, AstNodeId::from_ref(block));

        for stmt in block.stmts {
            match stmt {
                Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                Stmt::Item(item) => self.collect_item_from_ast(block_scope, item),
                Stmt::Expr { .. } => {}
            }
        }
    }

    /// For pattern `(a, S { b })`, this collects local bindings into the current scope:
    ///
    /// ```text
    /// scope
    /// ├─ DefKind::Local a
    /// └─ DefKind::Local b
    /// ```
    fn collect_pat(&mut self, scope: ScopeId, pat: &'cx Pat<'cx>) {
        match pat {
            Pat::Ident(pat) => {
                self.add_named_def(
                    scope,
                    DefKind::Local,
                    pat.ident.inner,
                    self.nearest_module_scope(scope),
                    AstNodeId::from_ref(pat),
                );
            }
            Pat::Reference(pat) => self.collect_pat(scope, pat.pat),
            Pat::Slice(pat) => {
                for elem in pat.elems {
                    self.collect_pat(scope, elem);
                }
            }
            Pat::Struct(pat) => {
                for field in pat.fields {
                    self.collect_pat(scope, field.pat);
                }
            }
            Pat::Tuple(pat) => {
                for elem in pat.elems {
                    self.collect_pat(scope, elem);
                }
            }
            Pat::Type(pat) => self.collect_pat(scope, pat.pat),
            Pat::Lit(_) | Pat::Path(_) | Pat::Rest(_) => {}
        }
    }

    /// Creates a `Generic` scope, then collects generic parameter defs.
    fn create_generic_scope(
        &mut self,
        parent_scope: ScopeId,
        generics: &Generics<'cx>,
    ) -> Option<ScopeId> {
        if generics.params.is_empty() {
            return None;
        }

        let generic_scope = self.db.add_scope(ScopeKind::Generic, Some(parent_scope));

        for param in generics.params {
            match param {
                GenericParam::Type(param) => {
                    self.add_named_def(
                        generic_scope,
                        DefKind::GenericType,
                        param.ident.inner,
                        self.nearest_module_scope(generic_scope),
                        AstNodeId::from_ref(param),
                    );
                }
                GenericParam::Const(param) => {
                    self.add_named_def(
                        generic_scope,
                        DefKind::GenericConst,
                        param.ident.inner,
                        self.nearest_module_scope(generic_scope),
                        AstNodeId::from_ref(param),
                    );
                }
                GenericParam::Unsupported(_) => {}
            }
        }

        Some(generic_scope)
    }

    fn create_function_body_scope(
        &mut self,
        parent_scope: ScopeId,
        sig: &'cx Signature<'cx>,
        block: &'cx Block<'cx>,
    ) -> ScopeId {
        let function_scope = self.db.add_scope(ScopeKind::Function, Some(parent_scope));

        for param in sig.params.iter().skip(1) {
            self.collect_pat(function_scope, param.pat.pat);
        }

        self.collect_block(function_scope, block);
        function_scope
    }

    fn add_named_def(
        &mut self,
        scope: ScopeId,
        kind: DefKind,
        name: Str<'cx>,
        visibility: ScopeId,
        ast_node: AstNodeId<'cx>,
    ) -> DefId {
        self.db
            .add_def(scope, kind, Some(name), visibility, Origin::Ast(ast_node))
    }

    fn visibility_from_ast(&self, scope: ScopeId, vis: &Visibility<'cx>) -> ScopeId {
        match vis {
            Visibility::Public(_) => NameDb::ROOT_SCOPE,
            Visibility::Restricted(path) => self.resolve_restricted_visibility_scope(scope, path),
            Visibility::Private => self.nearest_module_scope(scope),
        }
    }

    fn resolve_restricted_visibility_scope(&self, scope: ScopeId, path: &Path<'cx>) -> ScopeId {
        let mut scope = self.nearest_module_scope(scope);
        let mut segments = path.segments.iter();
        let first = segments
            .next()
            .expect("restricted visibility path must have at least one segment");

        match first.ident.inner.as_ref() {
            "crate" => scope = NameDb::CRATE_SCOPE,
            "self" => {}
            "super" => {
                scope = self
                    .parent_module_scope(scope)
                    .expect("restricted visibility `super` must have a parent module")
            }
            _ => panic!("restricted visibility path must start with `crate`, `self`, or `super`"),
        }

        for segment in segments {
            assert!(
                !segment.has_args(),
                "restricted visibility path segments must not have arguments"
            );

            let binding = self.db[scope]
                .bindings
                .get(Namespace::Type, segment.ident.inner)
                .expect("restricted visibility path segment must resolve");
            let def = binding
                .single()
                .expect("restricted visibility path segment must resolve unambiguously");
            let target = self.db.follow_aliases(def);
            scope = self.db[target]
                .scopes
                .path
                .expect("restricted visibility path segment must name a scope-bearing item");
        }

        scope
    }

    fn nearest_module_scope(&self, mut scope: ScopeId) -> ScopeId {
        loop {
            if matches!(self.db[scope].kind, ScopeKind::Crate | ScopeKind::Module) {
                return scope;
            }
            let Some(parent) = self.db[scope].parent else {
                return scope;
            };
            scope = parent;
        }
    }

    fn parent_module_scope(&self, scope: ScopeId) -> Option<ScopeId> {
        let mut scope = self.db[scope].parent?;
        loop {
            if matches!(self.db[scope].kind, ScopeKind::Crate | ScopeKind::Module) {
                return Some(scope);
            }
            scope = self.db[scope].parent?;
        }
    }
}
