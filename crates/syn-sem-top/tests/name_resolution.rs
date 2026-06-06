use std::fs;
use syn_sem_common::FilePath;
use syn_sem_name::{DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId, ScopeKind};
use syn_sem_top::TopCx;

#[test]
fn resolves_names_from_physical_module_files() {
    let tcx = TopCx::default();

    let entry_path = fixture("a1.rs");
    let entry_path = tcx.common.intern_path(&entry_path);
    let text = fs::read_to_string(&*entry_path).unwrap();
    let text = tcx.common.intern(&text);
    tcx.insert_virtual_file(entry_path, text).unwrap();

    let semantics = tcx.analyze(entry_path).unwrap();
    assert_fixture_modules(&tcx, semantics.names());
}

#[test]
fn resolves_names_from_virtual_module_files() {
    let tcx = TopCx::default();

    let entry_path = insert_virtual_fixture_tree(&tcx);

    let semantics = tcx.analyze(entry_path).unwrap();
    assert_fixture_modules(&tcx, semantics.names());
}

#[test]
fn resolves_local_use_paths_from_top_context() {
    let tcx = TopCx::default();
    let entry_path = tcx.common.intern("use_paths.rs");
    let text = tcx.common.intern(
        r#"
        mod a;

        mod b {
            use crate::a::Public;
            use crate::a::{self, Public as Renamed};

            mod inner {
                pub(super) struct Local;
            }

            use self::inner::Local;
            use super::a::Public as FromSuper;
        }
        "#,
    );
    tcx.insert_virtual_file(entry_path, text).unwrap();
    tcx.insert_virtual_file(
        tcx.common.intern("use_paths/a.rs"),
        tcx.common.intern("pub struct Public;"),
    )
    .unwrap();

    let semantics = tcx.analyze(entry_path).unwrap();
    let db = semantics.names();
    assert!(db
        .imports()
        .iter()
        .all(|import| import.status == ImportStatus::Resolved));

    let b_scope = module_scope(db, db.root_scope(), 1);
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "Public"),
        DefKind::Struct
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "Renamed"),
        DefKind::Struct
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "a"),
        DefKind::Module
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "Local"),
        DefKind::Struct
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "FromSuper"),
        DefKind::Struct
    );
}

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
    let a_scope = module_scope(db, root, 0);
    let child_scope = module_scope(db, a_scope, 0);
    let b_scope = module_scope(db, root, 1);

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
        resolve_result(&tcx, db, b_scope, Namespace::Type, "InA"),
        ResolveResult::NotFound
    );
    assert_eq!(db.imports()[3].status, ImportStatus::NotFound);
}

#[test]
#[should_panic(expected = "restricted visibility path must start with `crate`, `self`, or `super`")]
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

#[test]
fn imports_enum_variants_in_type_and_value_namespaces() {
    let tcx = TopCx::default();
    let entry_path = tcx.common.intern("variants.rs");
    let text = tcx.common.intern(
        r#"
        mod a {
            pub enum E {
                V,
            }
        }

        mod b {
            use crate::a::E::V;
        }
        "#,
    );
    tcx.insert_virtual_file(entry_path, text).unwrap();

    let semantics = tcx.analyze(entry_path).unwrap();
    let db = semantics.names();
    let b_scope = module_scope(db, db.root_scope(), 1);

    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Type, "V"),
        DefKind::Variant
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b_scope, Namespace::Value, "V"),
        DefKind::Variant
    );
}

fn insert_virtual_fixture_tree<'tcx>(tcx: &'tcx TopCx<'tcx>) -> FilePath<'tcx> {
    let entry_path = tcx.common.intern("a1.rs");
    let text = tcx.common.intern(include_str!("file/a1.rs"));
    tcx.insert_virtual_file(entry_path, text).unwrap();

    for (path, text) in [
        ("a1/b1.rs", include_str!("file/a1/b1.rs")),
        ("a1/b1/b2.rs", include_str!("file/a1/b1/b2.rs")),
        ("c1.rs", include_str!("file/c1.rs")),
        ("d1/d2.rs", include_str!("file/d1/d2.rs")),
        ("a1/e1/e2.rs", include_str!("file/a1/e1/e2.rs")),
        ("a1/e1/e4.rs", include_str!("file/a1/e1/e4.rs")),
    ] {
        let path = tcx.common.intern(path);
        let text = tcx.common.intern(text);
        tcx.insert_virtual_file(path, text).unwrap();
    }

    entry_path
}

fn assert_fixture_modules<'tcx>(tcx: &'tcx TopCx<'tcx>, db: &NameDb<'tcx>) {
    let root = db.root_scope();

    assert_eq!(
        resolve_kind(tcx, db, root, Namespace::Type, "b1"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, root, Namespace::Type, "c1"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, root, Namespace::Type, "dx"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, root, Namespace::Type, "e1"),
        DefKind::Module
    );

    let b1_scope = module_scope(db, root, 0);
    let dx_scope = module_scope(db, root, 2);
    let e1_scope = module_scope(db, root, 3);

    assert_eq!(
        resolve_kind(tcx, db, b1_scope, Namespace::Type, "b2"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, dx_scope, Namespace::Type, "d2"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, e1_scope, Namespace::Type, "e2"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(tcx, db, e1_scope, Namespace::Type, "e3"),
        DefKind::Module
    );
}

fn fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/file")
        .join(path)
}

fn resolve_kind<'tcx>(
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
    db[def].kind
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

fn resolve_result<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    db: &NameDb<'tcx>,
    scope: ScopeId,
    namespace: Namespace,
    name: &str,
) -> ResolveResult {
    resolve_lexical(db, scope, namespace, tcx.common.intern(name))
}

fn resolve_lexical(
    db: &NameDb<'_>,
    mut scope: ScopeId,
    namespace: Namespace,
    name: syn_sem_name::Name<'_>,
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

fn module_scope(db: &NameDb<'_>, parent: ScopeId, nth: usize) -> ScopeId {
    db.scopes()
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Module && scope.parent == Some(parent))
        .nth(nth)
        .unwrap()
        .id
}
