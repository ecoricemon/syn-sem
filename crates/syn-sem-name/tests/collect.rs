use syn_sem_ast as ast;
use syn_sem_common::{CommonCx, FilePath};
use syn_sem_name::{
    collect::NameCollector, AstNodeId, DefId, DefKind, ImportId, ImportKind, ImportStatus, Name,
    NameDb, Namespace, ResolveResult, ScopeId, ScopeKind,
};

struct TestCx {
    common: CommonCx,
}

impl TestCx {
    fn parse<'cx>(
        &'cx self,
        scx: &'cx ast::SyntaxCx<'cx>,
        file_path: &str,
        source_text: &str,
    ) -> ast::SourceInput<'cx> {
        let file_path = self.common.intern(file_path);
        let source_text = self.common.intern(source_text);
        scx.parse_virtual_file(file_path, source_text).unwrap();
        ast::SourceInput {
            file_path,
            file: scx.lookup_source(file_path).unwrap().ast(),
        }
    }
}

fn collect<'cx>(
    files: impl IntoIterator<Item = ast::SourceInput<'cx>>,
    entry_path: FilePath<'cx>,
) -> NameDb<'cx> {
    NameCollector::new(files).collect(entry_path).unwrap()
}

fn root_type<'cx>(db: &NameDb<'cx>, name: Name<'cx>) -> DefId {
    let ResolveResult::Found(def) = db.resolve_type_path(db.root_scope(), [name].into_iter())
    else {
        panic!("expected root type path {name:?} to resolve");
    };
    def
}

fn path_type<'cx>(db: &NameDb<'cx>, path: impl IntoIterator<Item = Name<'cx>>) -> DefId {
    let path = path.into_iter().collect::<Vec<_>>();
    let ResolveResult::Found(def) = db.resolve_type_path(db.root_scope(), path.into_iter()) else {
        panic!("expected type path to resolve");
    };
    def
}

fn direct_type_binding<'cx>(db: &NameDb<'cx>, scope: ScopeId, name: Name<'cx>) -> Option<DefId> {
    direct_binding(db, scope, Namespace::Type, name)
}

fn direct_value_binding<'cx>(db: &NameDb<'cx>, scope: ScopeId, name: Name<'cx>) -> Option<DefId> {
    direct_binding(db, scope, Namespace::Value, name)
}

fn direct_binding<'cx>(
    db: &NameDb<'cx>,
    scope: ScopeId,
    namespace: Namespace,
    name: Name<'cx>,
) -> Option<DefId> {
    db.binding(scope, namespace, name)
        .and_then(|binding| binding.iter().next())
}

fn scope(db: &NameDb<'_>, kind: ScopeKind, nth: usize) -> ScopeId {
    db.scopes_with_kind(kind).nth(nth).unwrap()
}

fn unique_child_scope(db: &NameDb<'_>, parent: ScopeId, kind: ScopeKind) -> ScopeId {
    let mut scopes = db.child_scopes(parent, kind);
    let scope = scopes.next().unwrap();
    assert!(
        scopes.next().is_none(),
        "expected exactly one {kind:?} child scope under {parent:?}"
    );
    scope
}

fn single_def(db: &NameDb<'_>, kind: DefKind) -> DefId {
    let mut defs = db.defs_with_kind(kind);
    let def = defs.next().unwrap();
    assert!(defs.next().is_none(), "expected exactly one {kind:?} def");
    def
}

fn module_scope<'cx>(db: &NameDb<'cx>, parent: ScopeId, name: Name<'cx>) -> ScopeId {
    let def = direct_type_binding(db, parent, name).expect("expected module binding");
    assert_eq!(db[def].kind, DefKind::Module);
    db.def_path_scope(def).unwrap()
}

fn import_for<'cx>(db: &NameDb<'cx>, scope: ScopeId, source_path: &[Name<'cx>]) -> ImportId {
    let mut imports = db.imports_matching(scope, source_path);
    let import = imports.next().unwrap();
    assert!(
        imports.next().is_none(),
        "expected exactly one import for {source_path:?} in {scope:?}"
    );
    import
}

fn follow_aliases_kind<'cx>(
    db: &NameDb<'cx>,
    scope: ScopeId,
    namespace: Namespace,
    name: Name<'cx>,
) -> Option<DefKind> {
    direct_binding(db, scope, namespace, name).map(|def| db[db.follow_aliases(def)].kind)
}

mod modules {
    use super::*;

    #[test]
    fn collects_inline_external_and_missing_module_items_from_prepared_ast() {
        // Proves prepared AST inputs collect inline modules, supplied external modules, and missing module stubs.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        struct Root;

        mod inline {
            struct Child;
        }

        mod external;

        mod missing;
        "#,
        );
        let external = tcx.parse(
            &scx,
            "src/external.rs",
            r#"
        pub struct FromExternal;
        "#,
        );

        let db = collect([entry, external], entry.file_path);

        let root = root_type(&db, tcx.common.intern("Root"));
        assert_eq!(db[root].kind, DefKind::Struct);

        let inline = root_type(&db, tcx.common.intern("inline"));
        assert_eq!(db[inline].kind, DefKind::Module);

        let inline_scope = db.def_path_scope(inline).unwrap();
        let child = direct_type_binding(&db, inline_scope, tcx.common.intern("Child"))
            .expect("inline::Child should be collected inside the module scope");
        assert_eq!(db[child].kind, DefKind::Struct);

        let external_item = path_type(
            &db,
            [
                tcx.common.intern("crate"),
                tcx.common.intern("external"),
                tcx.common.intern("FromExternal"),
            ],
        );
        assert_eq!(db[external_item].kind, DefKind::Struct);

        let missing_mod = path_type(
            &db,
            [tcx.common.intern("crate"), tcx.common.intern("missing")],
        );
        assert_eq!(db[missing_mod].kind, DefKind::Module);

        let module_scope = db.def_path_scope(missing_mod).unwrap();
        assert!(
            direct_type_binding(&db, module_scope, tcx.common.intern("FromExternal")).is_none(),
            "name collection must not synthesize or load missing external module contents"
        );
    }
}

mod imports {
    use super::*;

    #[test]
    fn collects_import_declarations_from_use_trees() {
        // Proves use trees collect single, rename, and glob import declarations.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        use a::{b, c as d, *};
        "#,
        );

        let db = collect([entry], entry.file_path);
        let root = db.root_scope();
        let a = tcx.common.intern("a");
        let b = tcx.common.intern("b");
        let c = tcx.common.intern("c");
        let d = tcx.common.intern("d");

        assert_eq!(db.import_count(), 3);
        let single = import_for(&db, root, &[a, b]);
        let renamed = import_for(&db, root, &[a, c]);
        let glob = import_for(&db, root, &[a]);
        assert_eq!(db[single].kind, ImportKind::Single);
        assert_eq!(db[renamed].kind, ImportKind::Rename(d));
        assert_eq!(db[glob].kind, ImportKind::Glob);

        let ast::Item::Use(item) = &entry.file.items[0] else {
            panic!("expected use item");
        };
        assert_eq!(
            db.imports_for_ast_node(AstNodeId::from_ref(item)),
            &[single, renamed, glob]
        );
    }
}

mod visibility {
    use super::*;

    #[test]
    fn applies_restricted_visibility_to_imports() {
        // Proves restricted visibility controls which imports resolve across module boundaries.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
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

        let db = collect([entry], entry.file_path);
        let root = db.root_scope();
        let a_scope = module_scope(&db, root, tcx.common.intern("a"));
        let child_scope = module_scope(&db, a_scope, tcx.common.intern("child"));
        let b_scope = module_scope(&db, root, tcx.common.intern("b"));

        assert_eq!(
            follow_aliases_kind(&db, child_scope, Namespace::Type, tcx.common.intern("InA")),
            Some(DefKind::Struct)
        );
        assert_eq!(
            follow_aliases_kind(
                &db,
                b_scope,
                Namespace::Type,
                tcx.common.intern("CrateVisible")
            ),
            Some(DefKind::Struct)
        );
        assert_eq!(
            follow_aliases_kind(
                &db,
                b_scope,
                Namespace::Type,
                tcx.common.intern("SuperVisible")
            ),
            Some(DefKind::Struct)
        );
        assert_eq!(
            follow_aliases_kind(&db, b_scope, Namespace::Type, tcx.common.intern("InA")),
            None
        );

        let import = import_for(
            &db,
            b_scope,
            &[
                tcx.common.intern("crate"),
                tcx.common.intern("a"),
                tcx.common.intern("InA"),
            ],
        );
        assert_eq!(db[import].status, ImportStatus::NotFound);
    }

    #[test]
    #[should_panic(
        expected = "restricted visibility path must start with `crate`, `self`, or `super`"
    )]
    fn invalid_restricted_visibility_anchor_panics() {
        // Proves invalid restricted-visibility anchors are rejected during collection.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        mod a {
            pub(in a) struct Invalid;
        }
        "#,
        );

        let _ = collect([entry], entry.file_path);
    }

    #[test]
    #[should_panic(expected = "restricted visibility path segment must resolve")]
    fn unresolved_restricted_visibility_path_panics() {
        // Proves unresolved restricted-visibility paths are rejected during collection.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        mod a {
            pub(in crate::missing) struct Invalid;
        }
        "#,
        );

        let _ = collect([entry], entry.file_path);
    }
}

mod members {
    use super::*;

    #[test]
    fn collects_trait_associated_type_as_member() {
        // Proves trait associated types are collected as type-namespace members.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        trait Iterator {
            type Item;
        }
        "#,
        );

        let db = collect([entry], entry.file_path);
        let iterator = root_type(&db, tcx.common.intern("Iterator"));
        let ResolveResult::Found(item) =
            db.member(iterator, Namespace::Type, tcx.common.intern("Item"))
        else {
            panic!("expected Iterator::Item to resolve as a member");
        };

        assert_eq!(db[iterator].kind, DefKind::Trait);
        assert_eq!(db[item].kind, DefKind::AssocType);
    }
}

mod scopes {
    use super::*;

    #[test]
    fn collects_def_scope_links_for_items_and_members() {
        // Proves collected definitions expose generic, path, body, trait, and impl scope links.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
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

        let db = collect([entry], entry.file_path);
        let root = db.root_scope();

        let s = direct_type_binding(&db, root, tcx.common.intern("S")).unwrap();
        assert_eq!(db[s].kind, DefKind::Struct);
        assert_eq!(
            db[db.def_generic_scope(s).unwrap()].kind,
            ScopeKind::Generic
        );
        assert!(db.def_path_scope(s).is_none());
        assert!(db.def_body_scope(s).is_none());

        let e = direct_type_binding(&db, root, tcx.common.intern("E")).unwrap();
        assert_eq!(db[e].kind, DefKind::Enum);
        let e_generic = db.def_generic_scope(e).unwrap();
        let e_path = db.def_path_scope(e).unwrap();
        assert_eq!(db[e_generic].kind, ScopeKind::Generic);
        assert_eq!(db[e_path].kind, ScopeKind::Item);
        assert_eq!(db[e_path].parent, Some(e_generic));
        let variant = direct_type_binding(&db, e_path, tcx.common.intern("V")).unwrap();
        assert_eq!(db[variant].kind, DefKind::Variant);

        let tr = direct_type_binding(&db, root, tcx.common.intern("Tr")).unwrap();
        assert_eq!(db[tr].kind, DefKind::Trait);
        let tr_generic = db.def_generic_scope(tr).unwrap();
        let tr_scope = unique_child_scope(&db, tr_generic, ScopeKind::Trait);
        let c = direct_value_binding(&db, tr_scope, tcx.common.intern("C")).unwrap();
        assert_eq!(db[c].kind, DefKind::AssocConst);
        let assoc = direct_type_binding(&db, tr_scope, tcx.common.intern("Assoc")).unwrap();
        assert_eq!(db[assoc].kind, DefKind::AssocType);
        assert_eq!(
            db[db.def_generic_scope(assoc).unwrap()].kind,
            ScopeKind::Generic
        );
        let m = direct_value_binding(&db, tr_scope, tcx.common.intern("m")).unwrap();
        assert_eq!(db[m].kind, DefKind::AssocFn);
        let m_generic = db.def_generic_scope(m).unwrap();
        let m_body = db.def_body_scope(m).unwrap();
        assert_eq!(db[m_body].kind, ScopeKind::Function);
        assert_eq!(db[m_body].parent, Some(m_generic));

        let impl_def = single_def(&db, DefKind::Impl);
        let impl_generic = db.def_generic_scope(impl_def).unwrap();
        let impl_scope = unique_child_scope(&db, impl_generic, ScopeKind::Impl);
        let make = direct_value_binding(&db, impl_scope, tcx.common.intern("make")).unwrap();
        assert_eq!(db[make].kind, DefKind::AssocFn);
        let make_generic = db.def_generic_scope(make).unwrap();
        let make_body = db.def_body_scope(make).unwrap();
        assert_eq!(db[make_body].kind, ScopeKind::Function);
        assert_eq!(db[make_body].parent, Some(make_generic));

        let f = direct_value_binding(&db, root, tcx.common.intern("f")).unwrap();
        assert_eq!(db[f].kind, DefKind::Fn);
        let f_generic = db.def_generic_scope(f).unwrap();
        let f_body = db.def_body_scope(f).unwrap();
        assert_eq!(db[f_body].kind, ScopeKind::Function);
        assert_eq!(db[f_body].parent, Some(f_generic));
    }

    #[test]
    fn keeps_function_generics_params_and_block_locals_in_separate_scopes() {
        // Proves function generics, params, and block locals occupy separate lexical scopes.
        let tcx = TestCx {
            common: CommonCx::default(),
        };
        let scx = ast::SyntaxCx::new(&tcx.common);
        let entry = tcx.parse(
            &scx,
            "src/lib.rs",
            r#"
        fn f<T>(x: T) {
            let y = x;
        }
        "#,
        );
        let ast::Item::Fn(item) = &entry.file.items[0] else {
            panic!("expected function item");
        };
        let block_node = AstNodeId::from_ref(&item.block);
        let ast::Pat::Ident(x_pat) = item.sig.params[1].pat.pat else {
            panic!("expected parameter ident pattern");
        };
        let ast::Stmt::Local(local) = &item.block.stmts[0] else {
            panic!("expected local statement");
        };
        let ast::Pat::Ident(y_pat) = &local.pat else {
            panic!("expected local ident pattern");
        };

        let db = collect([entry], entry.file_path);
        let f = direct_binding(
            &db,
            db.root_scope(),
            Namespace::Value,
            tcx.common.intern("f"),
        )
        .expect("function should be collected in the value namespace");
        let generic_scope = db.def_generic_scope(f).unwrap();
        let function_scope = db.def_body_scope(f).unwrap();
        let block_scope = scope(&db, ScopeKind::Block, 0);

        assert_eq!(db.scope_for_ast_node(block_node), Some(block_scope));
        assert_eq!(
            direct_type_binding(&db, generic_scope, tcx.common.intern("T")).map(|def| db[def].kind),
            Some(DefKind::GenericType)
        );
        assert_eq!(
            db.binding(function_scope, Namespace::Value, tcx.common.intern("x"))
                .and_then(|binding| binding.iter().next())
                .map(|def| db[def].kind),
            Some(DefKind::Local)
        );
        assert_eq!(
            db.def_for_ast_node(AstNodeId::from_ref(x_pat))
                .map(|def| db[def].kind),
            Some(DefKind::Local)
        );
        assert_eq!(
            db.binding(block_scope, Namespace::Value, tcx.common.intern("y"))
                .and_then(|binding| binding.iter().next())
                .map(|def| db[def].kind),
            Some(DefKind::Local)
        );
        assert_eq!(
            db.def_for_ast_node(AstNodeId::from_ref(y_pat))
                .map(|def| db[def].kind),
            Some(DefKind::Local)
        );
    }
}
