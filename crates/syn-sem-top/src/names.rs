use crate::TopCx;
use std::path::{Path, PathBuf};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, Result};
use syn_sem_name::{
    DefId, DefKind, ImportKind, Name, NameDb, Namespace, Origin, ScopeId, ScopeKind, Visibility,
};

pub(crate) struct NameCollector<'tcx> {
    tcx: &'tcx TopCx<'tcx>,
    db: NameDb<'tcx>,
}

impl<'tcx> NameCollector<'tcx> {
    pub(crate) fn new(tcx: &'tcx TopCx<'tcx>) -> Self {
        Self {
            tcx,
            db: NameDb::default(),
        }
    }

    pub(crate) fn collect(
        mut self,
        file_path: FilePath<'tcx>,
        file: &ast::File<'tcx>,
    ) -> Result<NameDb<'tcx>> {
        let root = self.db.root_scope();
        let path = ModulePath::from_entry_file(PathBuf::from(&*file_path));
        for item in file.items {
            self.collect_item_from_module_tree(root, item, &path)?;
        }
        self.db.resolve_imports();
        Ok(self.db)
    }

    /// Collects one item while walking a crate's module tree.
    ///
    /// Unlike [`Self::collect_item_from_ast`], this variant follows `mod foo;` declarations to
    /// their source files and continues collecting there.
    fn collect_item_from_module_tree(
        &mut self,
        scope: ScopeId,
        item: &ast::Item<'tcx>,
        path: &ModulePath,
    ) -> Result<()> {
        match item {
            ast::Item::Mod(item) => self.collect_mod_from_module_tree(scope, item, path),
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
    fn collect_item_from_ast(&mut self, scope: ScopeId, item: &ast::Item<'tcx>) {
        match item {
            ast::Item::Const(item) => self.collect_const(scope, item),
            ast::Item::Enum(item) => self.collect_enum(scope, item),
            ast::Item::Fn(item) => self.collect_fn(scope, item),
            ast::Item::Impl(item) => self.collect_impl(scope, item),
            ast::Item::Mod(item) => self.collect_mod_from_ast(scope, item),
            ast::Item::Struct(item) => self.collect_struct(scope, item),
            ast::Item::Trait(item) => self.collect_trait(scope, item),
            ast::Item::Type(item) => self.collect_type(scope, item),
            ast::Item::Use(item) => self.collect_use(scope, item),
        }
    }

    fn collect_mod_from_module_tree(
        &mut self,
        parent_scope: ScopeId,
        item: &ast::ItemMod<'tcx>,
        path: &ModulePath,
    ) -> Result<()> {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def =
            self.add_named_def(parent_scope, DefKind::Module, item.ident.inner, visibility);
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_path_scope(module_def, module_scope);
        let module_dir = path.child_dir(item);

        if let Some(items) = item.items {
            let path = ModulePath {
                source_file: path.source_file.clone(),
                module_dir,
            };
            for item in items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        } else if let Some(file_path) = path.child_file(self.tcx, item)? {
            let file = self.tcx.syntax.lookup_source(file_path)?.ast();
            let path = ModulePath {
                source_file: file_path.as_ref().into(),
                module_dir,
            };
            for item in file.items {
                self.collect_item_from_module_tree(module_scope, item, &path)?;
            }
        }

        Ok(())
    }

    fn collect_mod_from_ast(&mut self, parent_scope: ScopeId, item: &ast::ItemMod<'tcx>) {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let module_def =
            self.add_named_def(parent_scope, DefKind::Module, item.ident.inner, visibility);
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
    fn collect_const(&mut self, parent_scope: ScopeId, item: &ast::ItemConst<'tcx>) {
        // DefKind::Const
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        self.add_named_def(parent_scope, DefKind::Const, item.ident.inner, visibility);
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
    fn collect_enum(&mut self, parent_scope: ScopeId, item: &ast::ItemEnum<'tcx>) {
        // DefKind::Enum
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let enum_def =
            self.add_named_def(parent_scope, DefKind::Enum, item.ident.inner, visibility);

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
    fn collect_fn(&mut self, parent_scope: ScopeId, item: &ast::ItemFn<'tcx>) {
        // DefKind::Fn
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let fn_def =
            self.add_named_def(parent_scope, DefKind::Fn, item.sig.ident.inner, visibility);

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
    fn collect_struct(&mut self, parent_scope: ScopeId, item: &ast::ItemStruct<'tcx>) {
        // DefKind::Struct
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(parent_scope, DefKind::Struct, item.ident.inner, visibility);

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
    fn collect_trait(&mut self, parent_scope: ScopeId, item: &ast::ItemTrait<'tcx>) {
        // Trait Def
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let trait_def =
            self.add_named_def(parent_scope, DefKind::Trait, item.ident.inner, visibility);

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
    fn collect_trait_item(&mut self, trait_scope: ScopeId, item: &ast::TraitItem<'tcx>) {
        match item {
            ast::TraitItem::Const(item) => {
                let def = self.add_named_def(
                    trait_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    Visibility::Private,
                );
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
    fn collect_impl(&mut self, parent_scope: ScopeId, item: &ast::ItemImpl<'tcx>) {
        // DefKind::Impl
        let impl_def = self.db.add_def(
            parent_scope,
            DefKind::Impl,
            None,
            Visibility::Private,
            Origin::Untracked,
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
    fn collect_impl_item(&mut self, impl_scope: ScopeId, item: &ast::ImplItem<'tcx>) {
        match item {
            ast::ImplItem::Const(item) => {
                let def = self.add_named_def(
                    impl_scope,
                    DefKind::AssocConst,
                    item.ident.inner,
                    Visibility::Private,
                );
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
    fn collect_type(&mut self, parent_scope: ScopeId, item: &ast::ItemType<'tcx>) {
        // DefKind::TypeAlias
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let def = self.add_named_def(
            parent_scope,
            DefKind::TypeAlias,
            item.ident.inner,
            visibility,
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
    fn collect_use(&mut self, scope: ScopeId, item: &ast::ItemUse<'tcx>) {
        let visibility = self.visibility_from_ast(scope, &item.vis);
        self.collect_use_tree(scope, Vec::new(), &item.tree, visibility);
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
        prefix: Vec<Name<'tcx>>,
        tree: &ast::UseTree<'tcx>,
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
    fn collect_block(&mut self, parent_scope: ScopeId, block: &ast::Block<'tcx>) {
        // Block scope
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));

        for stmt in block.stmts {
            match stmt {
                ast::Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                ast::Stmt::Item(item) => self.collect_item_from_ast(block_scope, item),
                ast::Stmt::Expr(_) => {}
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
    fn collect_pat(&mut self, scope: ScopeId, pat: &ast::Pat<'tcx>) {
        match pat {
            ast::Pat::Ident(pat) => {
                self.add_named_def(scope, DefKind::Local, pat.ident.inner, Visibility::Private);
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
        generics: &ast::Generics<'tcx>,
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
        sig: &ast::Signature<'tcx>,
        block: &ast::Block<'tcx>,
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
        name: Name<'tcx>,
        visibility: Visibility,
    ) -> DefId {
        self.db
            .add_def(scope, kind, Some(name), visibility, Origin::Untracked)
    }

    fn visibility_from_ast(&self, scope: ScopeId, vis: &ast::Visibility<'tcx>) -> Visibility {
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
        path: &ast::Path<'tcx>,
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

fn path_attr(item: &ast::ItemMod<'_>) -> Option<PathBuf> {
    let item = syn::parse_str::<syn::ItemMod>(item.span.source_text()).ok()?;

    item.attrs.into_iter().find_map(|attr| {
        if !attr.path().is_ident("path") {
            return None;
        }

        let syn::Meta::NameValue(meta) = attr.meta else {
            return None;
        };
        let syn::Expr::Lit(expr) = meta.value else {
            return None;
        };
        let syn::Lit::Str(path) = expr.lit else {
            return None;
        };
        Some(PathBuf::from(path.value()))
    })
}

/// Tracks filesystem locations while walking a Rust module tree.
///
/// It records both the file currently being collected and the directory used to search for that
/// module's out-of-line child files, such as `foo.rs` or `foo/mod.rs`.
struct ModulePath {
    /// Source file currently being collected.
    source_file: PathBuf,

    /// Directory used to search for child modules declared from this module.
    module_dir: PathBuf,
}

impl ModulePath {
    fn from_entry_file(file_path: PathBuf) -> Self {
        let source_dir = file_path.parent().unwrap_or_else(|| Path::new(""));
        let stem = file_path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .expect("root module path must be a Rust source file path");
        let module_dir = match stem {
            "lib" | "main" | "mod" => source_dir.to_path_buf(),
            stem => source_dir.join(stem),
        };

        Self {
            source_file: file_path,
            module_dir,
        }
    }

    fn source_dir(&self) -> &Path {
        self.source_file.parent().unwrap_or_else(|| Path::new(""))
    }

    fn child_dir(&self, module: &ast::ItemMod<'_>) -> PathBuf {
        if let Some(path) = path_attr(module) {
            return self.resolve_attr_path(path);
        }

        self.module_dir.join(module.ident.inner.as_ref())
    }

    fn child_file<'tcx>(
        &self,
        tcx: &'tcx TopCx<'tcx>,
        module: &ast::ItemMod<'tcx>,
    ) -> Result<Option<FilePath<'tcx>>> {
        if let Some(path) = path_attr(module) {
            return self.find_child_file(
                tcx,
                [
                    self.module_dir.join(&path).as_ref(),
                    self.source_dir().join(path).as_ref(),
                ],
            );
        }

        let name = module.ident.inner.as_ref();
        self.find_child_file(
            tcx,
            [
                self.module_dir.join(format!("{name}.rs")).as_ref(),
                self.module_dir.join(name).join("mod.rs").as_ref(),
            ],
        )
    }

    fn find_child_file<'tcx, 'a, II, I>(
        &self,
        tcx: &'tcx TopCx<'tcx>,
        candidates: II,
    ) -> Result<Option<FilePath<'tcx>>>
    where
        II: IntoIterator<IntoIter = I>,
        I: Iterator<Item = &'a Path> + Clone,
    {
        let candidates: I = candidates.into_iter();

        for path in candidates.clone() {
            if let Some(file_path) = tcx.has_parsed(path) {
                return Ok(Some(file_path));
            }
        }

        for path in candidates {
            if path.is_file() {
                return tcx.read_physical_file(path).map(Some);
            }
        }

        Ok(None)
    }

    fn resolve_attr_path(&self, path: PathBuf) -> PathBuf {
        let module_relative = self.module_dir.join(&path);
        if module_relative.exists() {
            return module_relative;
        }

        self.source_dir().join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::TopCx;
    use syn_sem_name::{DefId, Import, ImportKind, ImportStatus, Namespace, ResolveResult};

    fn expect_kind(
        db: &NameDb<'_>,
        scope: ScopeId,
        namespace: Namespace,
        name: Name<'_>,
        kind: DefKind,
    ) {
        let ResolveResult::Found(def) = resolve_lexical(db, scope, namespace, name) else {
            panic!("expected {name:?} to resolve in {namespace:?}");
        };
        assert_eq!(db[def].kind, kind);
    }

    fn get_direct_def<'tcx>(
        tcx: &'tcx TopCx<'tcx>,
        db: &NameDb<'tcx>,
        scope: ScopeId,
        namespace: Namespace,
        name: &str,
    ) -> DefId {
        let name = tcx.common.intern(name);
        let binding = db
            .binding(scope, namespace, name)
            .unwrap_or_else(|| panic!("expected direct binding for {name:?} in {namespace:?}"));
        let mut defs = binding.iter();
        assert_eq!(defs.len(), 1);
        defs.next().unwrap()
    }

    fn get_unique_child_scope(db: &NameDb<'_>, parent: ScopeId, kind: ScopeKind) -> ScopeId {
        let mut scopes = db
            .scopes()
            .iter()
            .filter(|scope| scope.parent == Some(parent) && scope.kind == kind)
            .map(|scope| scope.id);
        let scope = scopes.next().unwrap();
        assert!(
            scopes.next().is_none(),
            "expected exactly one {kind:?} child scope under {parent:?}"
        );
        scope
    }

    fn get_single_def(db: &NameDb<'_>, kind: DefKind) -> DefId {
        let mut defs = db
            .defs()
            .iter()
            .filter(|def| def.kind == kind)
            .map(|def| def.id);
        let def = defs.next().unwrap();
        assert!(defs.next().is_none(), "expected exactly one {kind:?} def");
        def
    }

    fn get_import<'a, 'tcx>(
        tcx: &'tcx TopCx<'tcx>,
        db: &'a NameDb<'tcx>,
        scope: ScopeId,
        source_path: &[&str],
    ) -> &'a Import<'tcx> {
        let source_path: Vec<_> = source_path
            .iter()
            .map(|segment| tcx.common.intern(segment))
            .collect();
        let mut imports = db
            .imports()
            .iter()
            .filter(|import| import.scope == scope && import.source_path == source_path);
        let import = imports.next().unwrap();
        assert!(
            imports.next().is_none(),
            "expected exactly one import for {source_path:?} in {scope:?}"
        );
        import
    }

    fn get_module_scope<'tcx>(
        tcx: &'tcx TopCx<'tcx>,
        db: &NameDb<'tcx>,
        parent: ScopeId,
        name: &str,
    ) -> ScopeId {
        let def = get_direct_def(tcx, db, parent, Namespace::Type, name);
        assert_eq!(db[def].kind, DefKind::Module);
        db[def].scopes.path.unwrap()
    }

    fn follow_aliases_kind<'tcx>(
        tcx: &'tcx TopCx<'tcx>,
        db: &NameDb<'tcx>,
        scope: ScopeId,
        namespace: Namespace,
        name: &str,
    ) -> DefKind {
        let name = tcx.common.intern(name);
        let ResolveResult::Found(def) = resolve_lexical(db, scope, namespace, name) else {
            panic!("expected {name:?} to resolve in {namespace:?}");
        };
        db[db.follow_aliases(def)].kind
    }

    fn resolve_lexical(
        db: &NameDb<'_>,
        mut scope: ScopeId,
        namespace: Namespace,
        name: Name<'_>,
    ) -> ResolveResult {
        loop {
            if let Some(binding) = db.binding(scope, namespace, name) {
                let mut defs = binding.iter();
                return match defs.len() {
                    0 => ResolveResult::NotFound,
                    1 => ResolveResult::Found(defs.next().unwrap()),
                    _ => ResolveResult::Ambiguous(defs.collect()),
                };
            }

            let Some(parent) = db[scope].parent else {
                return ResolveResult::NotFound;
            };
            scope = parent;
        }
    }

    /// Verifies the top-level analysis path collects modules, generic parameters, function
    /// parameters, and block locals into a lexically searchable name database.
    #[test]
    fn collects_names_from_top_context() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("test.rs");
        let text = tcx.common.intern(
            r#"
            pub mod model {
                pub struct User<T> {
                    id: T,
                }
            }

            fn load<T, const N: usize>(user: T) {
                let current = user;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let db = semantics.names();

        let root = db.root_scope();
        let load_def = get_direct_def(&tcx, db, root, Namespace::Value, "load");
        assert_eq!(db[load_def].kind, DefKind::Fn);
        let function_scope = db[load_def].scopes.body.unwrap();
        let block = get_unique_child_scope(db, function_scope, ScopeKind::Block);

        expect_kind(
            db,
            root,
            Namespace::Type,
            tcx.common.intern("model"),
            DefKind::Module,
        );
        expect_kind(
            db,
            block,
            Namespace::Type,
            tcx.common.intern("T"),
            DefKind::GenericType,
        );
        expect_kind(
            db,
            block,
            Namespace::Value,
            tcx.common.intern("N"),
            DefKind::GenericConst,
        );
        expect_kind(
            db,
            block,
            Namespace::Value,
            tcx.common.intern("user"),
            DefKind::Local,
        );
        expect_kind(
            db,
            block,
            Namespace::Value,
            tcx.common.intern("current"),
            DefKind::Local,
        );
    }

    /// Verifies grouped `use` trees are flattened into single, rename, and glob import records
    /// before import resolution creates alias definitions.
    #[test]
    fn collects_import_declarations() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("test.rs");
        let text = tcx.common.intern(
            r#"
            use a::{b, c as d, *};
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let db = semantics.names();
        let root = db.root_scope();

        assert_eq!(db.imports().len(), 3);
        let ab = get_import(&tcx, db, root, &["a", "b"]);
        assert_eq!(ab.kind, ImportKind::Single);
        let ac = get_import(&tcx, db, root, &["a", "c"]);
        assert_eq!(ac.kind, ImportKind::Rename(tcx.common.intern("d")));
        let a = get_import(&tcx, db, root, &["a"]);
        assert_eq!(a.kind, ImportKind::Glob);
    }

    /// Verifies restricted visibility syntax is converted into the correct visibility scopes and
    /// applied when collected imports are resolved.
    #[test]
    fn applies_restricted_visibility_to_imports() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("visibility.rs");
        let text = tcx.common.intern(
            r#"
            mod a {
                pub(crate) struct CrateVisible;
                pub(super) struct SuperVisible;
                pub(in crate::a) struct InA;

                pub mod child {
                    use super::InA;
                }
            }

            mod b {
                use crate::a::CrateVisible;
                use crate::a::SuperVisible;
                use crate::a::InA;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let db = semantics.names();
        let root = db.root_scope();
        let a_scope = get_module_scope(&tcx, db, root, "a");
        let child_scope = get_module_scope(&tcx, db, a_scope, "child");
        let b_scope = get_module_scope(&tcx, db, root, "b");

        assert_eq!(
            follow_aliases_kind(&tcx, db, child_scope, Namespace::Type, "InA"),
            DefKind::Struct
        );
        assert_eq!(
            follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "CrateVisible"),
            DefKind::Struct
        );
        assert_eq!(
            follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "SuperVisible"),
            DefKind::Struct
        );
        assert_eq!(
            resolve_lexical(db, b_scope, Namespace::Type, tcx.common.intern("InA")),
            ResolveResult::NotFound
        );

        let in_a_import = get_import(&tcx, db, b_scope, &["crate", "a", "InA"]);
        assert_eq!(in_a_import.status, ImportStatus::NotFound);
    }

    /// Verifies `pub(in ...)` rejects anchors other than `crate`, `self`, and `super`.
    #[test]
    #[should_panic(
        expected = "restricted visibility path must start with `crate`, `self`, or `super`"
    )]
    fn invalid_restricted_visibility_anchor_panics() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("invalid_visibility.rs");
        let text = tcx.common.intern(
            r#"
            mod a {
                pub(in a) struct Invalid;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let _ = tcx.analyze(entry_path);
    }

    /// Verifies `pub(in ...)` rejects restricted visibility paths whose module segments do not
    /// resolve.
    #[test]
    #[should_panic(expected = "restricted visibility path segment must resolve")]
    fn unresolved_restricted_visibility_path_panics() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("unresolved_visibility.rs");
        let text = tcx.common.intern(
            r#"
            mod a {
                pub(in crate::missing) struct Invalid;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let _ = tcx.analyze(entry_path);
    }

    /// Verifies item and member definitions carry the expected `DefScopes` links for generic,
    /// path, and body scopes.
    #[test]
    fn collects_def_scope_links_for_items_and_members() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("def_scopes.rs");
        let text = tcx.common.intern(
            r#"
            struct S<T>;

            enum E<U> {
                V,
            }

            trait Tr<W> {
                const C: usize;
                type Assoc<X>;
                fn m<Y>(y: Y) {
                    let z = y;
                }
            }

            impl<Z> S<Z> {
                fn make<Q>(q: Q) {
                    let r = q;
                }
            }

            fn f<N>(n: N) {
                let local = n;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let db = semantics.names();
        let root = db.root_scope();

        let s_def = get_direct_def(&tcx, db, root, Namespace::Type, "S");
        assert_eq!(db[s_def].kind, DefKind::Struct);
        let s_generic_scope = db[s_def].scopes.generic.unwrap();
        assert_eq!(db[s_generic_scope].kind, ScopeKind::Generic);
        assert!(db[s_def].scopes.path.is_none());
        assert!(db[s_def].scopes.body.is_none());

        let e_def = get_direct_def(&tcx, db, root, Namespace::Type, "E");
        assert_eq!(db[e_def].kind, DefKind::Enum);
        let e_generic_scope = db[e_def].scopes.generic.unwrap();
        let e_path_scope = db[e_def].scopes.path.unwrap();
        assert_eq!(db[e_generic_scope].kind, ScopeKind::Generic);
        assert_eq!(db[e_path_scope].kind, ScopeKind::Item);
        assert_eq!(db[e_path_scope].parent, Some(e_generic_scope));
        let variant_def = get_direct_def(&tcx, db, e_path_scope, Namespace::Type, "V");
        assert_eq!(db[variant_def].kind, DefKind::Variant);

        let trait_def = get_direct_def(&tcx, db, root, Namespace::Type, "Tr");
        assert_eq!(db[trait_def].kind, DefKind::Trait);
        let trait_generic_scope = db[trait_def].scopes.generic.unwrap();
        let trait_scope = get_unique_child_scope(db, trait_generic_scope, ScopeKind::Trait);
        let c_def = get_direct_def(&tcx, db, trait_scope, Namespace::Value, "C");
        assert_eq!(db[c_def].kind, DefKind::AssocConst);
        let assoc_def = get_direct_def(&tcx, db, trait_scope, Namespace::Type, "Assoc");
        assert_eq!(db[assoc_def].kind, DefKind::AssocType);
        assert_eq!(
            db[db[assoc_def].scopes.generic.unwrap()].kind,
            ScopeKind::Generic
        );
        let m_def = get_direct_def(&tcx, db, trait_scope, Namespace::Value, "m");
        assert_eq!(db[m_def].kind, DefKind::AssocFn);
        let m_generic_scope = db[m_def].scopes.generic.unwrap();
        let m_function_scope = db[m_def].scopes.body.unwrap();
        assert_eq!(db[m_function_scope].kind, ScopeKind::Function);
        assert_eq!(db[m_function_scope].parent, Some(m_generic_scope));

        let impl_def = get_single_def(db, DefKind::Impl);
        let impl_generic_scope = db[impl_def].scopes.generic.unwrap();
        let impl_scope = get_unique_child_scope(db, impl_generic_scope, ScopeKind::Impl);
        let make_def = get_direct_def(&tcx, db, impl_scope, Namespace::Value, "make");
        assert_eq!(db[make_def].kind, DefKind::AssocFn);
        let make_generic_scope = db[make_def].scopes.generic.unwrap();
        let make_function_scope = db[make_def].scopes.body.unwrap();
        assert_eq!(db[make_function_scope].kind, ScopeKind::Function);
        assert_eq!(db[make_function_scope].parent, Some(make_generic_scope));

        let f_def = get_direct_def(&tcx, db, root, Namespace::Value, "f");
        assert_eq!(db[f_def].kind, DefKind::Fn);
        let f_generic_scope = db[f_def].scopes.generic.unwrap();
        let f_function_scope = db[f_def].scopes.body.unwrap();
        assert_eq!(db[f_function_scope].kind, ScopeKind::Function);
        assert_eq!(db[f_function_scope].parent, Some(f_generic_scope));
    }

    /// Verifies function parameters are direct bindings of the `Function` scope, while `let`
    /// bindings inside the body are direct bindings of the nested `Block` scope.
    #[test]
    fn collects_function_and_block_bindings_in_separate_scopes() {
        let tcx = TopCx::default();

        let entry_path = tcx.common.intern("function_scopes.rs");
        let text = tcx.common.intern(
            r#"
            fn f(x: i32) {
                let y = x;
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let db = semantics.names();
        let root = db.root_scope();

        let f_def = get_direct_def(&tcx, db, root, Namespace::Value, "f");
        assert_eq!(db[f_def].kind, DefKind::Fn);
        let function_scope = db[f_def].scopes.body.unwrap();
        let block_scope = get_unique_child_scope(db, function_scope, ScopeKind::Block);

        let x_def = get_direct_def(&tcx, db, function_scope, Namespace::Value, "x");
        assert_eq!(db[x_def].kind, DefKind::Local);
        let y_def = get_direct_def(&tcx, db, block_scope, Namespace::Value, "y");
        assert_eq!(db[y_def].kind, DefKind::Local);
        assert!(db
            .binding(function_scope, Namespace::Value, tcx.common.intern("y"))
            .is_none());
        assert!(db
            .binding(block_scope, Namespace::Value, tcx.common.intern("x"))
            .is_none());
    }
}
