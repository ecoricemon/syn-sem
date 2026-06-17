//! AST-aware collection into [`NameDb`].
//!
//! Callers provide already parsed files, so this module does not read source files or depend on a
//! top-level orchestration context.

use crate::{
    AstNodeId, DefId, DefKind, ImportId, ImportKind, Name, NameDb, Namespace, Origin, ScopeId,
    ScopeKind, Visibility,
};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, Result};

/// Collects name-resolution facts from prepared AST inputs.
pub struct NameCollector<'cx> {
    files: BTreeMap<PathBuf, ast::SourceInput<'cx>>,
    db: NameDb<'cx>,
}

impl<'cx> NameCollector<'cx> {
    /// Creates a collector from already parsed source inputs.
    pub fn new(files: impl IntoIterator<Item = ast::SourceInput<'cx>>) -> Self {
        Self {
            files: files
                .into_iter()
                .map(|input| (PathBuf::from(input.file_path.as_ref()), input))
                .collect(),
            db: NameDb::default(),
        }
    }

    /// Collects names from prepared AST inputs starting at `entry_path`.
    pub fn collect(mut self, entry_path: FilePath<'cx>) -> Result<NameDb<'cx>> {
        let file = self
            .files
            .get(Path::new(entry_path.as_ref()))
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("name collection input is missing entry file: {entry_path}"),
                )
            })?
            .file;
        let root = self.db.root_scope();
        let path = ast::ModulePath::from_entry_file(PathBuf::from(entry_path.as_ref()));
        for item in file.items {
            self.collect_item_from_module_tree(root, item, &path)?;
        }
        self.db.resolve_imports();
        Ok(self.db)
    }

    fn child_file(
        &self,
        path: &ast::ModulePath,
        module: &ast::ItemMod<'cx>,
    ) -> Option<ast::SourceInput<'cx>> {
        path.child_file_candidates(module)
            .into_iter()
            .find_map(|candidate| self.files.get(&candidate).copied())
    }

    /// Collects one item while walking a crate's module tree.
    ///
    /// Unlike [`Self::collect_item_from_ast`], this variant follows `mod foo;` declarations to
    /// their source files and continues collecting there.
    fn collect_item_from_module_tree(
        &mut self,
        scope: ScopeId,
        item: &'cx ast::Item<'cx>,
        path: &ast::ModulePath,
    ) -> Result<()> {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            ast::Item::Mod(item) => self.collect_mod_from_module_tree(scope, item, path, ast_node),
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
    fn collect_item_from_ast(&mut self, scope: ScopeId, item: &'cx ast::Item<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            ast::Item::Const(item) => self.collect_const(scope, item, ast_node),
            ast::Item::Enum(item) => self.collect_enum(scope, item, ast_node),
            ast::Item::Fn(item) => self.collect_fn(scope, item, ast_node),
            ast::Item::Impl(item) => self.collect_impl(scope, item, ast_node),
            ast::Item::Mod(item) => self.collect_mod_from_ast(scope, item, ast_node),
            ast::Item::Struct(item) => self.collect_struct(scope, item, ast_node),
            ast::Item::Trait(item) => self.collect_trait(scope, item, ast_node),
            ast::Item::Type(item) => self.collect_type(scope, item, ast_node),
            ast::Item::Use(item) => self.collect_use(scope, item),
        }
    }

    fn collect_mod_from_module_tree(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ast::ItemMod<'cx>,
        path: &ast::ModulePath,
        ast_node: AstNodeId<'cx>,
    ) -> Result<()> {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def =
            self.add_named_def(parent_scope, DefKind::Module, item.ident.inner, visibility);
        self.db.set_def_ast_node(module_def, ast_node);
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_path_scope(module_def, module_scope);

        if let Some(items) = item.items {
            let path = path.enter_inline_module(item);
            for item in items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        } else if let Some(input) = self.child_file(path, item) {
            let path = path.enter_external_module(item, PathBuf::from(input.file_path.as_ref()));
            for item in input.file.items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        }

        Ok(())
    }

    fn collect_mod_from_ast(
        &mut self,
        parent_scope: ScopeId,
        item: &'cx ast::ItemMod<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def =
            self.add_named_def(parent_scope, DefKind::Module, item.ident.inner, visibility);
        self.db.set_def_ast_node(module_def, ast_node);
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
        item: &'cx ast::ItemConst<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Const
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(parent_scope, DefKind::Const, item.ident.inner, visibility);
        self.db.set_def_ast_node(def, ast_node);
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
        item: &'cx ast::ItemEnum<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Enum
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let enum_def =
            self.add_named_def(parent_scope, DefKind::Enum, item.ident.inner, visibility);
        self.db.set_def_ast_node(enum_def, ast_node);

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
            let def = self.add_named_def(
                path_scope,
                DefKind::Variant,
                variant.ident.inner,
                visibility,
            );
            self.db.set_def_ast_node(def, AstNodeId::from_ref(variant));
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
        item: &'cx ast::ItemFn<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Fn
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let fn_def =
            self.add_named_def(parent_scope, DefKind::Fn, item.sig.ident.inner, visibility);
        self.db.set_def_ast_node(fn_def, ast_node);

        // Generic scope
        let generic_scope = self.create_generic_scope(parent_scope, &item.generics);
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
        item: &'cx ast::ItemStruct<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Struct
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(parent_scope, DefKind::Struct, item.ident.inner, visibility);
        self.db.set_def_ast_node(def, ast_node);

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
        item: &'cx ast::ItemTrait<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // Trait Def
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let trait_def =
            self.add_named_def(parent_scope, DefKind::Trait, item.ident.inner, visibility);
        self.db.set_def_ast_node(trait_def, ast_node);

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
    fn collect_trait_item(&mut self, trait_scope: ScopeId, item: &'cx ast::TraitItem<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            ast::TraitItem::Const(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
                if let Some(generic_scope) = self.create_generic_scope(trait_scope, &item.generics)
                {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
            ast::TraitItem::Fn(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocFn,
                    item.sig.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
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
            ast::TraitItem::Type(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocType,
                    item.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
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
        item: &'cx ast::ItemImpl<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::Impl
        let impl_def = self.db.add_def(
            parent_scope,
            DefKind::Impl,
            None,
            Visibility::Private,
            Origin::Untracked,
        );
        self.db.set_def_ast_node(impl_def, ast_node);

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
    fn collect_impl_item(&mut self, impl_scope: ScopeId, item: &'cx ast::ImplItem<'cx>) {
        let ast_node = AstNodeId::from_ref(item);
        match item {
            ast::ImplItem::Const(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
                if let Some(generic_scope) = self.create_generic_scope(impl_scope, &item.generics) {
                    self.db.set_generic_scope(def, generic_scope);
                }
            }
            ast::ImplItem::Fn(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocFn,
                    item.sig.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
                let generic_scope = self.create_generic_scope(impl_scope, &item.sig.generics);
                if let Some(generic_scope) = generic_scope {
                    self.db.set_generic_scope(def, generic_scope);
                }
                let function_parent_scope = generic_scope.unwrap_or(impl_scope);
                let function_scope =
                    self.create_function_body_scope(function_parent_scope, &item.sig, &item.block);
                self.db.set_body_scope(def, function_scope);
            }
            ast::ImplItem::Type(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocType,
                    item.ident.inner,
                    Visibility::Private,
                );
                self.db.set_def_ast_node(def, ast_node);
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
        item: &'cx ast::ItemType<'cx>,
        ast_node: AstNodeId<'cx>,
    ) {
        // DefKind::TypeAlias
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(
            parent_scope,
            DefKind::TypeAlias,
            item.ident.inner,
            visibility,
        );
        self.db.set_def_ast_node(def, ast_node);

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
    fn collect_use(&mut self, scope: ScopeId, item: &'cx ast::ItemUse<'cx>) {
        let visibility = self.visibility_from_ast(scope, &item.vis);
        let start = self.db.import_count();
        self.collect_use_tree(scope, Vec::new(), &item.tree, visibility);
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
        prefix: Vec<Name<'cx>>,
        tree: &'cx ast::UseTree<'cx>,
        visibility: Visibility,
    ) {
        match tree {
            ast::UseTree::Path(tree) => {
                let mut prefix = prefix;
                prefix.push(tree.ident.inner);
                self.collect_use_tree(scope, prefix, tree.tree, visibility);
            }
            ast::UseTree::Name(tree) => {
                let mut source_path = prefix;
                source_path.push(tree.ident.inner);
                self.db.add_import(
                    scope,
                    source_path,
                    ImportKind::Single,
                    visibility,
                    Origin::Untracked,
                );
            }
            ast::UseTree::Rename(tree) => {
                let mut source_path = prefix;
                source_path.push(tree.ident.inner);
                self.db.add_import(
                    scope,
                    source_path,
                    ImportKind::Rename(tree.rename.inner),
                    visibility,
                    Origin::Untracked,
                );
            }
            ast::UseTree::Glob(_) => {
                self.db.add_import(
                    scope,
                    prefix,
                    ImportKind::Glob,
                    visibility,
                    Origin::Untracked,
                );
            }
            ast::UseTree::Group(tree) => {
                for tree in tree.items {
                    self.collect_use_tree(scope, prefix.clone(), tree, visibility);
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
    fn collect_block(&mut self, parent_scope: ScopeId, block: &'cx ast::Block<'cx>) {
        // Block scope
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));
        self.db
            .set_scope_ast_node(block_scope, AstNodeId::from_ref(block));

        for stmt in block.stmts {
            match stmt {
                ast::Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                ast::Stmt::Item(item) => self.collect_item_from_ast(block_scope, item),
                ast::Stmt::Expr { .. } => {}
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
    fn collect_pat(&mut self, scope: ScopeId, pat: &'cx ast::Pat<'cx>) {
        match pat {
            ast::Pat::Ident(pat) => {
                let def =
                    self.add_named_def(scope, DefKind::Local, pat.ident.inner, Visibility::Private);
                self.db.set_def_ast_node(def, AstNodeId::from_ref(pat));
            }
            ast::Pat::Reference(pat) => self.collect_pat(scope, pat.pat),
            ast::Pat::Slice(pat) => {
                for elem in pat.elems {
                    self.collect_pat(scope, elem);
                }
            }
            ast::Pat::Struct(pat) => {
                for field in pat.fields {
                    self.collect_pat(scope, field.pat);
                }
            }
            ast::Pat::Tuple(pat) => {
                for elem in pat.elems {
                    self.collect_pat(scope, elem);
                }
            }
            ast::Pat::Type(pat) => self.collect_pat(scope, pat.pat),
            ast::Pat::Lit(_) | ast::Pat::Path(_) | ast::Pat::Rest(_) => {}
        }
    }

    /// Creates a `Generic` scope, then collects generic parameter defs.
    fn create_generic_scope(
        &mut self,
        parent_scope: ScopeId,
        generics: &ast::Generics<'cx>,
    ) -> Option<ScopeId> {
        if generics.params.is_empty() {
            return None;
        }

        let generic_scope = self.db.add_scope(ScopeKind::Generic, Some(parent_scope));

        for param in generics.params {
            match param {
                ast::GenericParam::Type(param) => {
                    self.add_named_def(
                        generic_scope,
                        DefKind::GenericType,
                        param.ident.inner,
                        Visibility::Private,
                    );
                }
                ast::GenericParam::Const(param) => {
                    self.add_named_def(
                        generic_scope,
                        DefKind::GenericConst,
                        param.ident.inner,
                        Visibility::Private,
                    );
                }
                ast::GenericParam::Unsupported(_) => {}
            }
        }

        Some(generic_scope)
    }

    fn create_function_body_scope(
        &mut self,
        parent_scope: ScopeId,
        sig: &'cx ast::Signature<'cx>,
        block: &'cx ast::Block<'cx>,
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
        name: Name<'cx>,
        visibility: Visibility,
    ) -> DefId {
        self.db
            .add_def(scope, kind, Some(name), visibility, Origin::Untracked)
    }

    fn visibility_from_ast(&self, scope: ScopeId, vis: &ast::Visibility<'cx>) -> Visibility {
        match vis {
            ast::Visibility::Public(_) => Visibility::Public,
            ast::Visibility::Restricted(path) => {
                Visibility::Restricted(self.resolve_restricted_visibility_scope(scope, path))
            }
            ast::Visibility::Private => Visibility::Private,
        }
    }

    fn resolve_restricted_visibility_scope(
        &self,
        scope: ScopeId,
        path: &ast::Path<'cx>,
    ) -> ScopeId {
        let mut scope = self.nearest_module_scope(scope);
        let mut segments = path.segments.iter();
        let first = segments
            .next()
            .expect("restricted visibility path must have at least one segment");

        match first.ident.inner.as_ref() {
            "crate" => scope = self.db.root_scope(),
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
            if matches!(
                self.db[scope].kind,
                ScopeKind::CrateRoot | ScopeKind::Module
            ) {
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
            if matches!(
                self.db[scope].kind,
                ScopeKind::CrateRoot | ScopeKind::Module
            ) {
                return Some(scope);
            }
            scope = self.db[scope].parent?;
        }
    }
}
