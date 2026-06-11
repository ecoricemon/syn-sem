use std::fs;

use syn_sem_name::{DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId};
use syn_sem_pr::{BodyKind, ItemKind};
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

fn resolve_lexical<'cx>(
    db: &NameDb<'cx>,
    mut scope: ScopeId,
    namespace: Namespace,
    name: syn_sem_name::Name<'cx>,
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

mod upper_phase_integration {
    use super::*;
    use syn_sem_infer::{InferDb, PathTypeResolution, PrimitiveType, Type};

    // Validates the intended upper-phase consumption pattern:
    // traverse source program shape through syn-sem-pr and query definition/scope facts through
    // syn-sem-name, without depending on syn-sem-ast directly.
    #[test]
    fn consumes_program_repr_and_name_facts_together() {
        let tcx = TopCx::default();
        let entry_path = tcx.common.intern("upper_phase.rs");
        let text = tcx.common.intern(
            r#"
            pub(crate) mod inner {
                pub fn helper() {}
            }

            struct Local;

            pub fn entry(x: Local, y: usize) -> Local {
                x
            }

            pub fn generic<T>(x: T) -> T {
                x
            }
            "#,
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let names = semantics.names();
        let infer = InferDb::analyze(&tcx.common, repr, names);

        let entry = repr
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "entry"))
            .expect("entry function should be represented");
        let ItemKind::Fn {
            signature, body, ..
        } = entry.kind
        else {
            panic!("entry should be represented as a function item");
        };
        let entry_def = entry
            .def
            .expect("function item should link to a definition");

        assert_eq!(names[entry_def].kind, DefKind::Fn);
        assert_eq!(repr[body].kind, BodyKind::Block);
        assert_eq!(repr[body].scope, names[entry_def].scopes.body);
        assert!(repr[signature]
            .types
            .iter()
            .all(|ty| infer.type_for_repr_type(*ty).is_some()));

        let inner = repr
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "inner"))
            .expect("inner module should be represented");
        let ItemKind::Mod {
            scope,
            items: inner_items,
            ..
        } = &inner.kind
        else {
            panic!("inner should be represented as a module item");
        };
        let inner_def = inner.def.expect("module item should link to a definition");

        assert_eq!(names[inner_def].kind, DefKind::Module);
        assert_eq!(*scope, names[inner_def].scopes.path);
        assert_eq!(inner_items.len(), 1);
        assert!(matches!(repr[inner_items[0]].kind, ItemKind::Fn { .. }));

        let local_def = repr
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Local"))
            .and_then(|item| item.def)
            .expect("Local struct should link to a definition");

        let local_ty = repr[signature].types[0];
        let infer_local_ty = infer
            .type_for_repr_type(local_ty)
            .expect("signature return type should be lowered");
        let Type::Path(path) = &infer[infer_local_ty] else {
            panic!("signature return type should lower to path type");
        };
        assert_eq!(path.resolution, PathTypeResolution::Nominal(local_def));

        let usize_ty = repr[signature].types[2];
        let infer_usize_ty = infer
            .type_for_repr_type(usize_ty)
            .expect("signature parameter type should be lowered");
        let Type::Primitive(primitive) = infer[infer_usize_ty] else {
            panic!("usize signature parameter should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::Usize);

        let generic = repr
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "generic"))
            .expect("generic function should be represented");
        let ItemKind::Fn {
            signature: generic_signature,
            ..
        } = generic.kind
        else {
            panic!("generic should be represented as a function item");
        };
        let generic_def = generic
            .def
            .expect("generic function should link to a definition");
        let generic_scope = names[generic_def]
            .scopes
            .generic
            .expect("generic function should have a generic scope");
        let t_name = tcx.common.intern("T");
        let t_def = names
            .binding(generic_scope, Namespace::Type, t_name)
            .and_then(|binding| binding.single())
            .expect("T should bind to one generic type parameter");

        let t_return_ty = repr[generic_signature].types[0];
        let infer_t_return_ty = infer
            .type_for_repr_type(t_return_ty)
            .expect("generic return type should be lowered");
        let Type::Path(path) = &infer[infer_t_return_ty] else {
            panic!("generic return type should lower to path type");
        };
        assert_eq!(path.resolution, PathTypeResolution::GenericParam(t_def));
    }
}
