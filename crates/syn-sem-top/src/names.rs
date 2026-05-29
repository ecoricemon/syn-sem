use crate::TopCx;
use std::path::{Path, PathBuf};
use syn_sem_ast as ast;
use syn_sem_common::{FilePath, Result};
use syn_sem_name::{
    DefId, DefKind, ImportKind, Name, NameDb, Origin, ScopeId, ScopeKind, Visibility,
};

/// Collects a [`NameDb`] from a semantic AST file.
pub fn collect_names<'cx>(file: &ast::File<'cx>) -> NameDb<'cx> {
    NameCollector::default().collect_file(file)
}

pub(crate) fn collect_names_in_top<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    file_path: FilePath<'tcx>,
    file: &ast::File<'tcx>,
) -> Result<NameDb<'tcx>> {
    let mut collector = NameCollector::default();
    let root = collector.db.root_scope();
    let path = ModulePath::root(PathBuf::from(&*file_path));
    for item in file.items {
        collector.collect_item_in_top(tcx, root, item, &path)?;
    }
    Ok(collector.db)
}

#[derive(Default)]
struct NameCollector<'tcx> {
    db: NameDb<'tcx>,
}

impl<'tcx> NameCollector<'tcx> {
    fn collect_file(mut self, file: &ast::File<'tcx>) -> NameDb<'tcx> {
        let root = self.db.root_scope();
        for item in file.items {
            self.collect_item(root, item);
        }
        self.db
    }

    fn collect_item(&mut self, scope: ScopeId, item: &ast::Item<'tcx>) {
        match item {
            ast::Item::Const(item) => {
                self.add_named(
                    scope,
                    DefKind::Const,
                    item.ident.inner,
                    ast_visibility(&item.vis),
                );
            }
            ast::Item::Enum(item) => self.collect_enum(scope, item),
            ast::Item::Fn(item) => self.collect_fn(scope, item, ast_visibility(&item.vis)),
            ast::Item::Impl(item) => self.collect_impl(scope, item),
            ast::Item::Mod(item) => self.collect_mod(scope, item),
            ast::Item::Struct(item) => {
                self.add_named(
                    scope,
                    DefKind::Struct,
                    item.ident.inner,
                    ast_visibility(&item.vis),
                );
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Trait(item) => self.collect_trait(scope, item),
            ast::Item::Type(item) => {
                self.add_named(
                    scope,
                    DefKind::TypeAlias,
                    item.ident.inner,
                    ast_visibility(&item.vis),
                );
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Use(item) => {
                self.collect_use_tree(scope, Vec::new(), &item.tree, ast_visibility(&item.vis));
            }
        }
    }

    fn collect_enum(&mut self, parent_scope: ScopeId, item: &ast::ItemEnum<'tcx>) {
        self.add_named(
            parent_scope,
            DefKind::Enum,
            item.ident.inner,
            ast_visibility(&item.vis),
        );
        let item_scope = self.db.add_scope(ScopeKind::Item, Some(parent_scope));
        self.collect_generics_into(item_scope, &item.generics);

        for variant in item.variants {
            self.add_named(
                item_scope,
                DefKind::Variant,
                variant.ident.inner,
                Visibility::Private,
            );
        }
    }

    fn collect_mod(&mut self, parent_scope: ScopeId, item: &ast::ItemMod<'tcx>) {
        self.add_named(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            ast_visibility(&item.vis),
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));

        if let Some(items) = item.items {
            for item in items {
                self.collect_item(module_scope, item);
            }
        }
    }

    /// Top-level collection differs from AST-only collection only for out-of-line modules:
    /// `mod foo;` may require loading and collecting names from another source file.
    fn collect_item_in_top(
        &mut self,
        tcx: &'tcx TopCx<'tcx>,
        scope: ScopeId,
        item: &ast::Item<'tcx>,
        path: &ModulePath,
    ) -> Result<()> {
        match item {
            ast::Item::Mod(item) => self.collect_mod_in_top(tcx, scope, item, path),
            _ => {
                self.collect_item(scope, item);
                Ok(())
            }
        }
    }

    fn collect_mod_in_top(
        &mut self,
        tcx: &'tcx TopCx<'tcx>,
        parent_scope: ScopeId,
        item: &ast::ItemMod<'tcx>,
        path: &ModulePath,
    ) -> Result<()> {
        self.add_named(
            parent_scope,
            DefKind::Module,
            item.ident.inner,
            ast_visibility(&item.vis),
        );
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));
        let module_dir = path.child_dir(item);

        if let Some(items) = item.items {
            let path = ModulePath {
                source_file: path.source_file.clone(),
                module_dir,
            };
            for item in items {
                self.collect_item_in_top(tcx, module_scope, item, &path)?;
            }
        } else if let Some(file_path) = path.child_file(tcx, item)? {
            let file = tcx.syntax.lookup_source(file_path)?.ast();
            let path = ModulePath {
                source_file: file_path.as_ref().into(),
                module_dir,
            };
            for item in file.items {
                self.collect_item_in_top(tcx, module_scope, item, &path)?;
            }
        }

        Ok(())
    }

    fn collect_trait(&mut self, parent_scope: ScopeId, item: &ast::ItemTrait<'tcx>) {
        self.add_named(
            parent_scope,
            DefKind::Trait,
            item.ident.inner,
            ast_visibility(&item.vis),
        );
        let trait_scope = self.db.add_scope(ScopeKind::Trait, Some(parent_scope));
        self.collect_generics_into(trait_scope, &item.generics);

        for item in item.items {
            match item {
                ast::TraitItem::Const(item) => {
                    self.add_named(
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
                    self.add_named(
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
            Origin::Synthetic,
        );

        let impl_scope = self.db.add_scope(ScopeKind::Impl, Some(parent_scope));
        self.collect_generics_into(impl_scope, &item.generics);

        for item in item.items {
            match item {
                ast::ImplItem::Const(item) => {
                    self.add_named(
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
                    self.add_named(
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
        self.add_named(parent_scope, kind, sig.ident.inner, visibility);
    }

    fn collect_block(&mut self, parent_scope: ScopeId, block: &ast::Block<'tcx>) {
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));

        for stmt in block.stmts {
            match stmt {
                ast::Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                ast::Stmt::Item(item) => self.collect_item(block_scope, item),
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
                    self.add_named(
                        scope,
                        DefKind::TypeParam,
                        param.ident.inner,
                        Visibility::Private,
                    );
                }
                ast::GenericParam::Const(param) => {
                    self.add_named(
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
                self.add_named(scope, DefKind::Local, pat.ident.inner, Visibility::Private);
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
                    Origin::Synthetic,
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
                    Origin::Synthetic,
                );
            }
            ast::UseTree::Glob(_) => {
                self.db.add_import(
                    scope,
                    prefix,
                    ImportKind::Glob,
                    visibility,
                    Origin::Synthetic,
                );
            }
            ast::UseTree::Group(tree) => {
                for tree in tree.items {
                    self.collect_use_tree(scope, prefix.clone(), tree, visibility);
                }
            }
        }
    }

    fn add_named(
        &mut self,
        scope: ScopeId,
        kind: DefKind,
        name: Name<'tcx>,
        visibility: Visibility,
    ) -> DefId {
        self.db
            .add_def(scope, kind, Some(name), visibility, Origin::Synthetic)
    }
}

fn ast_visibility(vis: &ast::Visibility<'_>) -> Visibility {
    match vis {
        ast::Visibility::Public(_) => Visibility::Public,
        ast::Visibility::Restricted(_) | ast::Visibility::Private => Visibility::Private,
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

struct ModulePath {
    source_file: PathBuf,
    module_dir: PathBuf,
}

impl ModulePath {
    fn root(file_path: PathBuf) -> Self {
        let source_dir = file_path.parent().unwrap_or_else(|| Path::new(""));
        let module_dir = match file_path.file_stem().and_then(|stem| stem.to_str()) {
            Some("lib" | "main" | "mod") => source_dir.to_path_buf(),
            Some(stem) => source_dir.join(stem),
            None => source_dir.to_path_buf(),
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
        let ResolveResult::Found(def) = db.resolve_lexical(scope, namespace, name) else {
            panic!("expected {name:?} to resolve in {namespace:?}");
        };
        assert_eq!(db[def].kind, kind);
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
