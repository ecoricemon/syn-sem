use syn_sem_hir::ItemKind;
use syn_sem_name::{AstNodeId, DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId};
use syn_sem_top::TopCx;

/// Verifies physical module files are loaded from the filesystem and `use` declarations across
/// those files resolve to the expected definitions.
#[test]
fn resolves_imports_from_physical_module_files() {
    let tcx = TopCx::default();

    let entry_path = fixture("a1.rs");

    let semantics = tcx.analyze_file(entry_path).unwrap();
    assert!(!semantics.hir().files().is_empty());
    assert!(!semantics.hir().items().is_empty());

    let db = semantics.names();
    let root = db.root_scope();

    assert!(db
        .import_ids()
        .all(|import| db[import].status == ImportStatus::Resolved));

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
    use syn_sem_hir as hir;
    use syn_sem_infer as infer;

    // Validates the intended upper-phase consumption pattern:
    // traverse HIR source spine through syn-sem-hir and query definition/scope facts through
    // syn-sem-name, without depending on syn-sem-ast directly.
    #[test]
    fn consumes_hir_and_name_facts_together() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "upper_phase.rs",
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
            )
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = infer::InferDb::analyze(&tcx.common, hir, names);

        let entry = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "entry"))
            .expect("entry function should be represented");
        let ItemKind::Fn {
            signature, block, ..
        } = entry.kind
        else {
            panic!("entry should be represented as a function item");
        };
        let entry_def = entry
            .def
            .expect("function item should link to a definition");

        assert_eq!(names[entry_def].kind, DefKind::Fn);
        assert_eq!(
            hir[block].scope,
            names.scope_for_ast_node(AstNodeId::from_ref(hir[block].block))
        );
        assert_ne!(hir[block].scope, names[entry_def].scopes.body);
        assert!(hir[signature]
            .params
            .iter()
            .map(|param| param.tid)
            .all(|ty| infer.type_for_hir_type(ty).is_some()));

        let inner = hir
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
        assert!(matches!(hir[inner_items[0]].kind, ItemKind::Fn { .. }));

        let local_def = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Local"))
            .and_then(|item| item.def)
            .expect("Local struct should link to a definition");

        let local_tid = hir[signature].params[0].tid;
        let infer_local_tid = infer
            .type_for_hir_type(local_tid)
            .expect("signature return type should be lowered");
        assert_eq!(infer.nominal_def(infer_local_tid), Some(local_def));

        let usize_tid = hir[signature].params[2].tid;
        let infer_usize_tid = infer
            .type_for_hir_type(usize_tid)
            .expect("signature parameter type should be lowered");
        let infer::Type::Primitive(primitive) = infer[infer_usize_tid] else {
            panic!("usize signature parameter should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::Usize);

        let generic = hir
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

        let t_return_tid = hir[generic_signature].params[0].tid;
        let infer_t_return_tid = infer
            .type_for_hir_type(t_return_tid)
            .expect("generic return type should be lowered");
        assert_eq!(infer.generic_param_def(infer_t_return_tid), Some(t_def));
    }

    #[test]
    fn consumes_projection_normalization_query_from_hir() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "projection_normalization.rs",
                r#"
            struct Vec;

            trait Iterator {
                type Item;
            }

            impl Iterator for Vec {
                type Item = u32;
            }

            struct Output {
                field: <Vec as Iterator>::Item,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = infer::InferDb::analyze(&tcx.common, hir, names);

        let output = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Output"))
            .expect("Output struct should be represented");
        let ItemKind::Struct { fields, .. } = &output.kind else {
            panic!("Output should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("Output should have one field");
        };
        let field_tid = hir[*field].tid;
        assert_eq!(hir[field_tid].source, hir::TypeSource::StructField);

        let projection = infer
            .type_for_hir_type(field_tid)
            .expect("Output.field type should be lowered");
        let infer::ProjectionType {
            assoc_type,
            self_tid,
            trait_tid,
        } = infer
            .projection(projection)
            .expect("Output.field should remain a projection path");
        assert!(self_tid.is_some());
        assert!(trait_tid.is_some());
        assert_eq!(names[*assoc_type].kind, DefKind::AssocType);

        let infer::ProjectionNormalizationResult::Known(normalized_tid) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_tid] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::U32);
    }

    #[test]
    fn consumes_generic_projection_normalization_query_from_hir() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "generic_projection_normalization.rs",
                r#"
            struct Vec<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Output {
                field: <Vec<u32> as Iterator>::Item,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let infer = infer::InferDb::analyze(&tcx.common, hir, semantics.names());

        let output = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Output"))
            .expect("Output struct should be represented");
        let ItemKind::Struct { fields, .. } = &output.kind else {
            panic!("Output should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("Output should have one field");
        };
        let field_tid = hir[*field].tid;
        let projection = infer
            .type_for_hir_type(field_tid)
            .expect("Output.field type should be lowered");
        assert!(infer.projection(projection).is_some());

        let infer::ProjectionNormalizationResult::Known(normalized_tid) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_tid] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::U32);
    }

    #[test]
    fn normalizes_projection_with_multiple_generic_bindings() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "multi_generic_projection_normalization.rs",
                r#"
            struct Pair<K, V>;

            trait Select {
                type Output;
            }

            impl<K, V> Select for Pair<K, V> {
                type Output = V;
            }

            struct Result {
                field: <Pair<u32, bool> as Select>::Output,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let infer = infer::InferDb::analyze(&tcx.common, hir, semantics.names());

        let result = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Result"))
            .expect("Result struct should be represented");
        let ItemKind::Struct { fields, .. } = &result.kind else {
            panic!("Result should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("Result should have one field");
        };
        let field_tid = hir[*field].tid;

        let projection = infer
            .type_for_hir_type(field_tid)
            .expect("Result.field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(normalized_tid) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_tid] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::Bool);
    }

    #[test]
    fn normalizes_nested_generic_projection_values() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "nested_generic_projection_normalization.rs",
                r#"
            struct Vec<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            struct Result {
                field: <Vec<u32> as Wrap>::Output,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let infer = infer::InferDb::analyze(&tcx.common, hir, semantics.names());

        let result = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Result"))
            .expect("Result struct should be represented");
        let ItemKind::Struct { fields, .. } = &result.kind else {
            panic!("Result should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("Result should have one field");
        };
        let field_tid = hir[*field].tid;

        let projection = infer
            .type_for_hir_type(field_tid)
            .expect("Result.field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(normalized_tid) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let infer::Type::Path(path) = &infer[normalized_tid] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<u32> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [infer::GenericArg::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<u32> should have one type argument");
        };
        let infer::Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::U32);
    }

    #[test]
    fn consumes_recursive_normalization_query_from_hir() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "recursive_projection_normalization.rs",
                r#"
            struct Vec<T>;
            struct Option<T>;

            trait Iterator {
                type Item;
            }

            impl<T> Iterator for Vec<T> {
                type Item = T;
            }

            struct Result {
                field: Option<<Vec<u32> as Iterator>::Item>,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let mut infer = infer::InferDb::analyze(&tcx.common, hir, semantics.names());

        let result = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Result"))
            .expect("Result struct should be represented");
        let ItemKind::Struct { fields, .. } = &result.kind else {
            panic!("Result should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("Result should have one field");
        };
        let field_tid = hir[*field].tid;

        let normalized_tid = infer
            .normalized_type_for_hir_type(field_tid)
            .expect("Result.field type should be lowered");
        assert_option_of(&infer, normalized_tid, infer::PrimitiveType::U32);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "contextual_projection_substitution.rs",
                r#"
            struct Vec<T>;
            struct Box<T>;
            struct Option<T>;

            trait Wrap {
                type Output;
            }

            impl<T> Wrap for Vec<T> {
                type Output = Option<T>;
            }

            impl<U> Wrap for Box<U> {
                type Output = Option<U>;
            }

            struct Result {
                vec_field: <Vec<u32> as Wrap>::Output,
                box_field: <Box<bool> as Wrap>::Output,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let infer = infer::InferDb::analyze(&tcx.common, hir, semantics.names());

        let result = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "Result"))
            .expect("Result struct should be represented");
        let ItemKind::Struct { fields, .. } = &result.kind else {
            panic!("Result should be represented as a struct item");
        };
        let [vec_field, box_field] = fields.as_slice() else {
            panic!("Result should have two fields");
        };

        let vec_field_tid = hir[*vec_field].tid;
        let box_field_tid = hir[*box_field].tid;

        let vec_projection = infer
            .type_for_hir_type(vec_field_tid)
            .expect("Result.vec_field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(vec_normalized_tid) =
            infer.projection_normalization(vec_projection)
        else {
            panic!("Vec projection should have one context-matched normalization");
        };
        assert_option_of(&infer, vec_normalized_tid, infer::PrimitiveType::U32);

        let box_projection = infer
            .type_for_hir_type(box_field_tid)
            .expect("Result.box_field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(box_normalized_tid) =
            infer.projection_normalization(box_projection)
        else {
            panic!("Box projection should have one context-matched normalization");
        };
        assert_option_of(&infer, box_normalized_tid, infer::PrimitiveType::Bool);
    }

    fn assert_option_of(
        infer: &infer::InferDb<'_>,
        tid: infer::TypeId,
        expected: infer::PrimitiveType,
    ) {
        let infer::Type::Path(path) = &infer[tid] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [infer::GenericArg::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<T> should have one type argument");
        };
        let infer::Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, expected);
    }
}
