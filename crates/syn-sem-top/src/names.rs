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
            ast::Item::Const(item) => {
                self.add_named_def(
                    scope,
                    DefKind::Const,
                    item.ident.inner,
                    self.visibility_from_ast(scope, &item.vis),
                );
            }
            ast::Item::Enum(item) => self.collect_enum(scope, item),
            ast::Item::Fn(item) => {
                self.collect_fn(scope, item, self.visibility_from_ast(scope, &item.vis))
            }
            ast::Item::Impl(item) => self.collect_impl(scope, item),
            ast::Item::Mod(item) => self.collect_mod_from_ast(scope, item),
            ast::Item::Struct(item) => {
                self.add_named_def(
                    scope,
                    DefKind::Struct,
                    item.ident.inner,
                    self.visibility_from_ast(scope, &item.vis),
                );
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Trait(item) => self.collect_trait(scope, item),
            ast::Item::Type(item) => {
                self.add_named_def(
                    scope,
                    DefKind::TypeAlias,
                    item.ident.inner,
                    self.visibility_from_ast(scope, &item.vis),
                );
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Use(item) => {
                self.collect_use_tree(
                    scope,
                    Vec::new(),
                    &item.tree,
                    self.visibility_from_ast(scope, &item.vis),
                );
            }
        }
    }

    fn collect_mod_from_module_tree(
        &mut self,
        parent_scope: ScopeId,
        item: &ast::ItemMod<'tcx>,
        path: &ModulePath,
    ) -> Result<()> {
        let module_def = self.add_named_def(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            self.visibility_from_ast(parent_scope, &item.vis),
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_child_scope(module_def, module_scope);
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
        let module_def = self.add_named_def(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            self.visibility_from_ast(parent_scope, &item.vis),
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        self.db.set_child_scope(module_def, module_scope);

        if let Some(items) = item.items {
            for item in items {
                self.collect_item_from_ast(module_scope, item);
            }
        }
    }

    fn collect_enum(&mut self, parent_scope: ScopeId, item: &ast::ItemEnum<'tcx>) {
        let visibility = self.visibility_from_ast(parent_scope, &item.vis);
        let enum_def =
            self.add_named_def(parent_scope, DefKind::Enum, item.ident.inner, visibility);
        let item_scope = self.db.add_scope(ScopeKind::Item, Some(parent_scope));
        self.db.set_child_scope(enum_def, item_scope);
        self.collect_generics_into(item_scope, &item.generics);

        for variant in item.variants {
            self.add_named_def(
                item_scope,
                DefKind::Variant,
                variant.ident.inner,
                visibility,
            );
        }
    }

    fn collect_trait(&mut self, parent_scope: ScopeId, item: &ast::ItemTrait<'tcx>) {
        self.add_named_def(
            parent_scope,
            DefKind::Trait,
            item.ident.inner,
            self.visibility_from_ast(parent_scope, &item.vis),
        );
        let trait_scope = self.db.add_scope(ScopeKind::Trait, Some(parent_scope));
        self.collect_generics_into(trait_scope, &item.generics);

        for item in item.items {
            match item {
                ast::TraitItem::Const(item) => {
                    self.add_named_def(
                        trait_scope,
                        DefKind::Const,
                        item.ident.inner,
                        Visibility::Private,
                    );
                    self.collect_generics(trait_scope, &item.generics);
                }
                ast::TraitItem::Fn(item) => {
                    self.collect_fn_signature(
                        trait_scope,
                        DefKind::Fn,
                        &item.sig,
                        Visibility::Private,
                    );
                    if let Some(block) = &item.default {
                        self.collect_block(trait_scope, block);
                    }
                }
                ast::TraitItem::Type(item) => {
                    self.add_named_def(
                        trait_scope,
                        DefKind::TypeAlias,
                        item.ident.inner,
                        Visibility::Private,
                    );
                    self.collect_generics(trait_scope, &item.generics);
                }
            }
        }
    }

    fn collect_impl(&mut self, parent_scope: ScopeId, item: &ast::ItemImpl<'tcx>) {
        self.db.add_def(
            parent_scope,
            DefKind::Impl,
            None,
            Visibility::Private,
            Origin::Untracked,
        );

        let impl_scope = self.db.add_scope(ScopeKind::Impl, Some(parent_scope));
        self.collect_generics_into(impl_scope, &item.generics);

        for item in item.items {
            match item {
                ast::ImplItem::Const(item) => {
                    self.add_named_def(
                        impl_scope,
                        DefKind::Const,
                        item.ident.inner,
                        Visibility::Private,
                    );
                    self.collect_generics(impl_scope, &item.generics);
                }
                ast::ImplItem::Fn(item) => {
                    self.collect_fn_signature(
                        impl_scope,
                        DefKind::Fn,
                        &item.sig,
                        Visibility::Private,
                    );
                    self.collect_block(impl_scope, &item.block);
                }
                ast::ImplItem::Type(item) => {
                    self.add_named_def(
                        impl_scope,
                        DefKind::TypeAlias,
                        item.ident.inner,
                        Visibility::Private,
                    );
                    self.collect_generics(impl_scope, &item.generics);
                }
            }
        }
    }

    fn collect_fn(
        &mut self,
        parent_scope: ScopeId,
        item: &ast::ItemFn<'tcx>,
        visibility: Visibility,
    ) {
        self.collect_fn_signature(parent_scope, DefKind::Fn, &item.sig, visibility);

        let generic_scope = self
            .db
            .add_scope(ScopeKind::GenericParams, Some(parent_scope));
        self.collect_generics_into(generic_scope, &item.generics);

        let body_scope = self
            .db
            .add_scope(ScopeKind::FunctionBody, Some(generic_scope));

        for param in item.sig.params.iter().skip(1) {
            self.collect_pat(body_scope, param.pat.pat);
        }

        self.collect_block(body_scope, &item.block);
    }

    fn collect_fn_signature(
        &mut self,
        parent_scope: ScopeId,
        kind: DefKind,
        sig: &ast::Signature<'tcx>,
        visibility: Visibility,
    ) {
        self.add_named_def(parent_scope, kind, sig.ident.inner, visibility);
    }

    fn collect_block(&mut self, parent_scope: ScopeId, block: &ast::Block<'tcx>) {
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));

        for stmt in block.stmts {
            match stmt {
                ast::Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                ast::Stmt::Item(item) => self.collect_item_from_ast(block_scope, item),
                ast::Stmt::Expr(_) => {}
            }
        }
    }

    fn collect_generics(&mut self, parent_scope: ScopeId, generics: &ast::Generics<'tcx>) {
        let generic_scope = self
            .db
            .add_scope(ScopeKind::GenericParams, Some(parent_scope));
        self.collect_generics_into(generic_scope, generics);
    }

    fn collect_generics_into(&mut self, scope: ScopeId, generics: &ast::Generics<'tcx>) {
        for param in generics.params {
            match param {
                ast::GenericParam::Type(param) => {
                    self.add_named_def(
                        scope,
                        DefKind::TypeParam,
                        param.ident.inner,
                        Visibility::Private,
                    );
                }
                ast::GenericParam::Const(param) => {
                    self.add_named_def(
                        scope,
                        DefKind::ConstParam,
                        param.ident.inner,
                        Visibility::Private,
                    );
                }
                ast::GenericParam::Unsupported(_) => {}
            }
        }
    }

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
                .child_scope
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
    use syn_sem_name::{ImportKind, Namespace, ResolveResult};

    fn scope(db: &NameDb<'_>, kind: ScopeKind, nth: usize) -> ScopeId {
        db.scopes()
            .iter()
            .filter(|scope| scope.kind == kind)
            .nth(nth)
            .unwrap()
            .id
    }

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
        let block = scope(db, ScopeKind::Block, 0);

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
            DefKind::TypeParam,
        );
        expect_kind(
            db,
            block,
            Namespace::Value,
            tcx.common.intern("N"),
            DefKind::ConstParam,
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

        assert_eq!(db.imports().len(), 3);
        assert_eq!(
            db.imports()[0].source_path,
            vec![tcx.common.intern("a"), tcx.common.intern("b")]
        );
        assert_eq!(db.imports()[0].kind, ImportKind::Single);
        assert_eq!(
            db.imports()[1].source_path,
            vec![tcx.common.intern("a"), tcx.common.intern("c")]
        );
        assert_eq!(
            db.imports()[1].kind,
            ImportKind::Rename(tcx.common.intern("d"))
        );
        assert_eq!(db.imports()[2].source_path, vec![tcx.common.intern("a")]);
        assert_eq!(db.imports()[2].kind, ImportKind::Glob);
    }
}
