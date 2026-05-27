use syn_sem_ast as ast;
use syn_sem_common::{CommonCx, SourceText};
use syn_sem_name::{
    DefId, DefKind, Name, NameDb, Namespace, Origin, ResolveResult, ScopeId, ScopeKind, Visibility,
};

#[derive(Default)]
struct AstNameCollector<'cx> {
    db: NameDb<'cx>,
}

impl<'cx> AstNameCollector<'cx> {
    fn collect_file(mut self, file: &ast::File<'cx>) -> NameDb<'cx> {
        let root = self.db.root_scope();
        self.collect_items(root, file.items);
        self.db
    }

    fn collect_items(&mut self, scope: ScopeId, items: &[ast::Item<'cx>]) {
        for item in items {
            self.collect_item(scope, item);
        }
    }

    fn collect_item(&mut self, scope: ScopeId, item: &ast::Item<'cx>) {
        match item {
            ast::Item::Const(item) => {
                self.add_named(scope, DefKind::Const, item.ident.inner);
            }
            ast::Item::Enum(item) => {
                self.add_named(scope, DefKind::Enum, item.ident.inner);
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Fn(item) => self.collect_fn(scope, item),
            ast::Item::Mod(item) => self.collect_mod(scope, item),
            ast::Item::Struct(item) => {
                self.add_named(scope, DefKind::Struct, item.ident.inner);
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Trait(item) => {
                self.add_named(scope, DefKind::Trait, item.ident.inner);
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Type(item) => {
                self.add_named(scope, DefKind::TypeAlias, item.ident.inner);
                self.collect_generics(scope, &item.generics);
            }
            ast::Item::Impl(_) | ast::Item::Use(_) => {}
        }
    }

    fn collect_mod(&mut self, parent_scope: ScopeId, item: &ast::ItemMod<'cx>) {
        self.add_named(parent_scope, DefKind::Module, item.ident.inner);
        let module_scope = self.db.add_scope(ScopeKind::Module, Some(parent_scope));

        if let Some(items) = item.items {
            self.collect_items(module_scope, items);
        }
    }

    fn collect_fn(&mut self, parent_scope: ScopeId, item: &ast::ItemFn<'cx>) {
        self.add_named(parent_scope, DefKind::Fn, item.sig.ident.inner);

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

    fn collect_block(&mut self, parent_scope: ScopeId, block: &ast::Block<'cx>) {
        let block_scope = self.db.add_scope(ScopeKind::Block, Some(parent_scope));

        for stmt in block.stmts {
            match stmt {
                ast::Stmt::Local(local) => self.collect_pat(block_scope, &local.pat),
                ast::Stmt::Item(item) => self.collect_item(block_scope, item),
                ast::Stmt::Expr(_) => {}
            }
        }
    }

    fn collect_generics(&mut self, parent_scope: ScopeId, generics: &ast::Generics<'cx>) {
        let generic_scope = self
            .db
            .add_scope(ScopeKind::GenericParams, Some(parent_scope));
        self.collect_generics_into(generic_scope, generics);
    }

    fn collect_generics_into(&mut self, scope: ScopeId, generics: &ast::Generics<'cx>) {
        for param in generics.params {
            match param {
                ast::GenericParam::Type(param) => {
                    self.add_named(scope, DefKind::TypeParam, param.ident.inner);
                }
                ast::GenericParam::Const(param) => {
                    self.add_named(scope, DefKind::ConstParam, param.ident.inner);
                }
                ast::GenericParam::Unsupported(_) => {}
            }
        }
    }

    fn collect_pat(&mut self, scope: ScopeId, pat: &ast::Pat<'cx>) {
        match pat {
            ast::Pat::Ident(pat) => {
                self.add_named(scope, DefKind::Local, pat.ident.inner);
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

    fn add_named(&mut self, scope: ScopeId, kind: DefKind, name: Name<'cx>) -> DefId {
        self.db.add_def(
            scope,
            kind,
            Some(name),
            Visibility::Private,
            Origin::Synthetic,
        )
    }
}

fn collect_names<'cx>(scx: &'cx ast::SyntaxCx<'cx>, text: SourceText<'cx>) -> NameDb<'cx> {
    let file = parse_file(scx, text);
    AstNameCollector::default().collect_file(&file)
}

fn parse_file<'cx>(scx: &'cx ast::SyntaxCx<'cx>, text: SourceText<'cx>) -> ast::File<'cx> {
    let file_path = scx.common.intern("test.rs");
    scx.parse_virtual_file(file_path, text).unwrap();
    let source = scx.get_source(file_path).unwrap();
    source.ast().clone()
}

fn scope(db: &NameDb<'_>, kind: ScopeKind, nth: usize) -> ScopeId {
    db.scopes()
        .iter()
        .filter(|scope| scope.kind == kind)
        .nth(nth)
        .unwrap()
        .id
}

fn expect_def(
    db: &NameDb<'_>,
    scope: ScopeId,
    namespace: Namespace,
    name: Name<'_>,
    kind: DefKind,
) -> DefId {
    let ResolveResult::Found(def) = db.resolve_lexical(scope, namespace, name) else {
        panic!("expected {name:?} to resolve in {namespace:?}");
    };
    assert_eq!(db[def].kind, kind);
    def
}

#[test]
fn resolves_function_generics_params_and_locals_from_ast() {
    let ccx = CommonCx::new();
    let scx = ast::SyntaxCx::new(&ccx);
    let text = ccx.intern(
        r#"
        fn f<T, const N: usize>(x: T) {
            let y = x;
        }
        "#,
    );
    let db = collect_names(&scx, text);

    let generic_scope = scope(&db, ScopeKind::GenericParams, 0);
    let block_scope = scope(&db, ScopeKind::Block, 0);

    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::TypeParam,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("N"),
        DefKind::ConstParam,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("x"),
        DefKind::Local,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("y"),
        DefKind::Local,
    );
    expect_def(
        &db,
        generic_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::TypeParam,
    );
}

#[test]
fn resolves_local_item_declared_inside_function_from_ast() {
    let ccx = CommonCx::new();
    let scx = ast::SyntaxCx::new(&ccx);
    let text = ccx.intern(
        r#"
        fn f<T>(x: T) {
            struct Local<U> {
                value: U,
            }

            let y: Local<T>;
        }
        "#,
    );
    let db = collect_names(&scx, text);

    let block_scope = scope(&db, ScopeKind::Block, 0);

    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("Local"),
        DefKind::Struct,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::TypeParam,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("x"),
        DefKind::Local,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("y"),
        DefKind::Local,
    );
}

#[test]
fn keeps_type_and_value_namespaces_separate_from_ast() {
    let ccx = CommonCx::new();
    let scx = ast::SyntaxCx::new(&ccx);
    let text = ccx.intern(
        r#"
        fn f<T>(T: i32) {
            let x: T = T;
        }
        "#,
    );
    let db = collect_names(&scx, text);

    let block_scope = scope(&db, ScopeKind::Block, 0);

    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::TypeParam,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("T"),
        DefKind::Local,
    );
}
