use std::fs;

use syn_sem_name::{DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId};
use syn_sem_pr::ItemKind;
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
    use syn_sem_infer::{
        GenericArgument, InferDb, PrimitiveType, ProjectionNormalizationResult, ProjectionType,
        Type, TypeId,
    };
    use syn_sem_pr::TypeSource;

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
            signature, block, ..
        } = entry.kind
        else {
            panic!("entry should be represented as a function item");
        };
        let entry_def = entry
            .def
            .expect("function item should link to a definition");

        assert_eq!(names[entry_def].kind, DefKind::Fn);
        assert_eq!(repr[block].scope, names[entry_def].scopes.body);
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
        assert_eq!(infer.nominal_def(infer_local_ty), Some(local_def));

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
        assert_eq!(infer.generic_param_def(infer_t_return_ty), Some(t_def));
    }

    #[test]
    fn consumes_projection_normalization_query_from_program_repr() {
        let tcx = TopCx::default();
        let entry_path = tcx.common.intern("projection_normalization.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let names = semantics.names();
        let infer = InferDb::analyze(&tcx.common, repr, names);

        let output = repr
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
        let field_ty = repr[*field].ty;
        assert_eq!(repr[field_ty].source, TypeSource::StructField);

        let projection = infer
            .type_for_repr_type(field_ty)
            .expect("Output.field type should be lowered");
        let ProjectionType {
            assoc_type,
            self_ty,
            trait_ty,
        } = infer
            .projection(projection)
            .expect("Output.field should remain a projection path");
        assert!(self_ty.is_some());
        assert!(trait_ty.is_some());
        assert_eq!(names[*assoc_type].kind, DefKind::AssocType);

        let ProjectionNormalizationResult::Known(normalized_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn consumes_generic_projection_normalization_query_from_program_repr() {
        let tcx = TopCx::default();
        let entry_path = tcx.common.intern("generic_projection_normalization.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let infer = InferDb::analyze(&tcx.common, repr, semantics.names());

        let output = repr
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
        let field_ty = repr[*field].ty;
        let projection = infer
            .type_for_repr_type(field_ty)
            .expect("Output.field type should be lowered");
        assert!(infer.projection(projection).is_some());

        let ProjectionNormalizationResult::Known(normalized_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn normalizes_projection_with_multiple_generic_bindings() {
        let tcx = TopCx::default();
        let entry_path = tcx
            .common
            .intern("multi_generic_projection_normalization.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let infer = InferDb::analyze(&tcx.common, repr, semantics.names());

        let result = repr
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
        let field_ty = repr[*field].ty;

        let projection = infer
            .type_for_repr_type(field_ty)
            .expect("Result.field type should be lowered");
        let ProjectionNormalizationResult::Known(normalized_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::Bool);
    }

    #[test]
    fn normalizes_nested_generic_projection_values() {
        let tcx = TopCx::default();
        let entry_path = tcx
            .common
            .intern("nested_generic_projection_normalization.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let infer = InferDb::analyze(&tcx.common, repr, semantics.names());

        let result = repr
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
        let field_ty = repr[*field].ty;

        let projection = infer
            .type_for_repr_type(field_ty)
            .expect("Result.field type should be lowered");
        let ProjectionNormalizationResult::Known(normalized_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let Type::Path(path) = &infer[normalized_ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<u32> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArgument::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<u32> should have one type argument");
        };
        let Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn consumes_recursive_normalization_query_from_program_repr() {
        let tcx = TopCx::default();
        let entry_path = tcx.common.intern("recursive_projection_normalization.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let mut infer = InferDb::analyze(&tcx.common, repr, semantics.names());

        let result = repr
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
        let field_ty = repr[*field].ty;

        let normalized_ty = infer
            .normalized_type_for_repr_type(field_ty)
            .expect("Result.field type should be lowered");
        assert_option_of(&infer, normalized_ty, PrimitiveType::U32);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
        let tcx = TopCx::default();
        let entry_path = tcx.common.intern("contextual_projection_substitution.rs");
        let text = tcx.common.intern(
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
        );

        tcx.insert_virtual_file(entry_path, text).unwrap();
        let semantics = tcx.analyze(entry_path).unwrap();
        let repr = semantics.repr();
        let infer = InferDb::analyze(&tcx.common, repr, semantics.names());

        let result = repr
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

        let vec_field_ty = repr[*vec_field].ty;
        let box_field_ty = repr[*box_field].ty;

        let vec_projection = infer
            .type_for_repr_type(vec_field_ty)
            .expect("Result.vec_field type should be lowered");
        let ProjectionNormalizationResult::Known(vec_normalized_ty) =
            infer.projection_normalization(vec_projection)
        else {
            panic!("Vec projection should have one context-matched normalization");
        };
        assert_option_of(&infer, vec_normalized_ty, PrimitiveType::U32);

        let box_projection = infer
            .type_for_repr_type(box_field_ty)
            .expect("Result.box_field type should be lowered");
        let ProjectionNormalizationResult::Known(box_normalized_ty) =
            infer.projection_normalization(box_projection)
        else {
            panic!("Box projection should have one context-matched normalization");
        };
        assert_option_of(&infer, box_normalized_ty, PrimitiveType::Bool);
    }

    fn assert_option_of(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        let Type::Path(path) = &infer[ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArgument::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<T> should have one type argument");
        };
        let Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, expected);
    }
}
