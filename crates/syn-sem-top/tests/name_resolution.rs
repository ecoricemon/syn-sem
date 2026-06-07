use std::fs;

use syn_sem_name::{DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId};
use syn_sem_top::TopCx;

/// Verifies physical module files are loaded from the filesystem and `use` declarations across
/// those files resolve to the expected definitions.
#[test]
fn resolves_imports_from_physical_module_files() {
    let tcx = TopCx::default();

    let entry_path = fixture("a1.rs");
    let entry_path = tcx.common.intern_path(&entry_path);
    let text = fs::read_to_string(&*entry_path).unwrap();
    let text = tcx.common.intern(&text);
    tcx.insert_virtual_file(entry_path, text).unwrap();

    let semantics = tcx.analyze(entry_path).unwrap();
    assert!(!semantics.repr().files().is_empty());
    assert!(!semantics.repr().items().is_empty());

    let db = semantics.names();
    let root = db.root_scope();

    assert!(db
        .imports()
        .iter()
        .all(|import| import.status == ImportStatus::Resolved));

    assert_eq!(
        resolve_kind(&tcx, db, root, Namespace::Type, "b1"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(&tcx, db, root, Namespace::Type, "c1"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(&tcx, db, root, Namespace::Type, "dx"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(&tcx, db, root, Namespace::Type, "e1"),
        DefKind::Module
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, root, Namespace::Type, "FromB1"),
        DefKind::Struct
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, root, Namespace::Type, "FromC1"),
        DefKind::Struct
    );

    let b1_scope = get_module_scope(&tcx, db, root, "b1");
    let dx_scope = get_module_scope(&tcx, db, root, "dx");
    let e1_scope = get_module_scope(&tcx, db, root, "e1");

    assert_eq!(
        resolve_kind(&tcx, db, b1_scope, Namespace::Type, "b2"),
        DefKind::Module
    );
    assert_eq!(
        follow_aliases_kind(&tcx, db, b1_scope, Namespace::Type, "FromB2"),
        DefKind::Struct
    );
    assert_eq!(
        resolve_kind(&tcx, db, dx_scope, Namespace::Type, "d2"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(&tcx, db, e1_scope, Namespace::Type, "e2"),
        DefKind::Module
    );
    assert_eq!(
        resolve_kind(&tcx, db, e1_scope, Namespace::Type, "e3"),
        DefKind::Module
    );
}

fn fixture(path: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/file")
        .join(path)
}

fn get_module_scope<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    db: &NameDb<'tcx>,
    parent: ScopeId,
    name: &str,
) -> ScopeId {
    let def = resolve_def(tcx, db, parent, Namespace::Type, name);
    assert_eq!(db[def].kind, DefKind::Module);
    db[def].scopes.path.unwrap()
}

fn resolve_kind<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    db: &NameDb<'tcx>,
    scope: ScopeId,
    namespace: Namespace,
    name: &str,
) -> DefKind {
    let def = resolve_def(tcx, db, scope, namespace, name);
    db[def].kind
}

fn resolve_def<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    db: &NameDb<'tcx>,
    scope: ScopeId,
    namespace: Namespace,
    name: &str,
) -> syn_sem_name::DefId {
    let name = tcx.common.intern(name);
    let ResolveResult::Found(def) = resolve_lexical(db, scope, namespace, name) else {
        panic!("expected {name:?} to resolve in {namespace:?}");
    };
    def
}

fn follow_aliases_kind<'tcx>(
    tcx: &'tcx TopCx<'tcx>,
    db: &NameDb<'tcx>,
    scope: ScopeId,
    namespace: Namespace,
    name: &str,
) -> DefKind {
    let def = resolve_def(tcx, db, scope, namespace, name);
    db[db.follow_aliases(def)].kind
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
