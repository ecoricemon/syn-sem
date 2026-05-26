use syn_sem_common::FilePath;
use syn_sem_name::{DefKind, NameDb, Namespace, ResolveResult, ScopeId, ScopeKind};
use syn_sem_top::TopCx;

#[test]
fn resolves_names_from_physical_module_files() {
    let tcx = TopCx::default();
    let entry_path = fixture("a1.rs");
    let entry_source = std::fs::read_to_string(&entry_path).unwrap();
    let entry_file = tcx
        .insert_virtual_file(entry_path.to_str().unwrap(), &entry_source)
        .unwrap();

    let semantics = tcx.analyze(entry_file).unwrap();

    assert_fixture_modules(&tcx, semantics.names());
}

#[test]
fn resolves_names_from_virtual_module_files() {
    let tcx = TopCx::default();
    let entry_file = insert_virtual_fixture_tree(&tcx);
    let semantics = tcx.analyze(entry_file).unwrap();

    assert_fixture_modules(&tcx, semantics.names());
}

fn insert_virtual_fixture_tree<'tcx>(tcx: &'tcx TopCx<'tcx>) -> FilePath<'tcx> {
    let entry_file = tcx
        .insert_virtual_file("a1.rs", include_str!("file/a1.rs"))
        .unwrap();

    for (path, code) in [
        ("a1/b1.rs", include_str!("file/a1/b1.rs")),
        ("a1/b1/b2.rs", include_str!("file/a1/b1/b2.rs")),
        ("c1.rs", include_str!("file/c1.rs")),
        ("d1/d2.rs", include_str!("file/d1/d2.rs")),
        ("a1/e1/e2.rs", include_str!("file/a1/e1/e2.rs")),
        ("a1/e1/e4.rs", include_str!("file/a1/e1/e4.rs")),
    ] {
        tcx.insert_virtual_file(path, code).unwrap();
    }

    entry_file
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
    let name = tcx.intern(name);
    let ResolveResult::Found(def) = db.resolve_lexical(scope, namespace, name) else {
        panic!("expected {name:?} to resolve in {namespace:?}");
    };
    db[def].kind
}

fn module_scope(db: &NameDb<'_>, parent: ScopeId, nth: usize) -> ScopeId {
    db.scopes()
        .iter()
        .filter(|scope| scope.kind == ScopeKind::Module && scope.parent == Some(parent))
        .nth(nth)
        .unwrap()
        .id
}
