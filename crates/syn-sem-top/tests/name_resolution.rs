use syn_sem_common::Str;
use syn_sem_eval::ConstValue;
use syn_sem_hir::{ArrayLen, ItemKind, TypeKind};
use syn_sem_name::{
    AstNodeId, DefId, DefKind, ImportStatus, NameDb, Namespace, ResolveResult, ScopeId,
};
use syn_sem_top::TopCx;

mod files {
    use super::*;

    /// Verifies physical module files are loaded from the filesystem and `use` declarations across
    /// those files resolve to the expected definitions.
    #[test]
    fn resolves_imports_from_physical_module_files() {
        // Proves physical module files load and resolve cross-file imports.
        let tcx = TopCx::default();

        let entry_path = fixture("a1.rs");

        let semantics = tcx.analyze_file(entry_path).unwrap();
        assert!(!semantics.hir().files().is_empty());
        assert!(!semantics.hir().items().is_empty());

        let db = semantics.names();
        let crate_scope = NameDb::CRATE_SCOPE;

        assert!(db
            .import_ids()
            .all(|import| db[import].status == ImportStatus::Resolved));

        assert_eq!(
            resolve_kind(&tcx, db, crate_scope, Namespace::Type, "b1"),
            DefKind::Module
        );
        assert_eq!(
            resolve_kind(&tcx, db, crate_scope, Namespace::Type, "c1"),
            DefKind::Module
        );
        assert_eq!(
            resolve_kind(&tcx, db, crate_scope, Namespace::Type, "dx"),
            DefKind::Module
        );
        assert_eq!(
            resolve_kind(&tcx, db, crate_scope, Namespace::Type, "e1"),
            DefKind::Module
        );
        assert_eq!(
            follow_aliases_kind(&tcx, db, crate_scope, Namespace::Type, "FromB1"),
            DefKind::Struct
        );
        assert_eq!(
            follow_aliases_kind(&tcx, db, crate_scope, Namespace::Type, "FromC1"),
            DefKind::Struct
        );

        let b1_scope = get_module_scope(&tcx, db, crate_scope, "b1");
        let dx_scope = get_module_scope(&tcx, db, crate_scope, "dx");
        let e1_scope = get_module_scope(&tcx, db, crate_scope, "e1");

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

    #[test]
    fn evaluates_constants_from_physical_modules_and_entry_array_lengths() {
        let tcx = TopCx::default();
        let semantics = tcx.analyze_file(fixture("a1.rs")).unwrap();

        let c1_scope = get_module_scope(&tcx, semantics.names(), NameDb::CRATE_SCOPE, "c1");
        let capacity = resolve_def(
            &tcx,
            semantics.names(),
            c1_scope,
            Namespace::Value,
            "CAPACITY",
        );
        assert_eq!(semantics.names()[capacity].kind, DefKind::Const);

        let c1_file = semantics
            .hir()
            .files()
            .iter()
            .find(|file| file.file_path.as_ref().ends_with("/c1.rs"))
            .expect("c1.rs should have a HIR file");
        assert_eq!(c1_file.scope, Some(c1_scope));
        assert!(c1_file.items.iter().any(|item| {
            let item = &semantics.hir()[*item];
            item.def == Some(capacity) && item.parent_scope == Some(c1_scope)
        }));

        let c1_module = semantics
            .hir()
            .items()
            .iter()
            .find(|item| matches!(item.name, Some(name) if name.as_ref() == "c1"))
            .expect("c1 module should have a HIR item");
        let ItemKind::Mod {
            external_file: Some(external_file),
            ..
        } = c1_module.kind
        else {
            panic!("c1 module should link to its external HIR file");
        };
        assert_eq!(external_file, c1_file.id);

        assert!(
            semantics.eval().value_for_const_def(capacity).is_some(),
            "CAPACITY should be evaluated from the physical module file"
        );

        let buffer = semantics
            .hir()
            .items()
            .iter()
            .find(|item| matches!(item.name, Some(name) if name.as_ref() == "SizedBuffer"))
            .expect("entry type alias should have a HIR item");
        let ItemKind::Type { ty, .. } = buffer.kind else {
            panic!("SizedBuffer should be a type alias");
        };
        let TypeKind::Array {
            len: ArrayLen::Expr(len),
            ..
        } = semantics.hir()[ty].kind
        else {
            panic!("SizedBuffer should retain its array-length expression");
        };

        assert!(matches!(
            semantics.eval().value_for_hir_expr(len),
            Some(ConstValue::Int(value)) if value.value == 5
        ));

        let buffer = semantics
            .hir()
            .items()
            .iter()
            .find(|item| matches!(item.name, Some(name) if name.as_ref() == "NestedBuffer"))
            .expect("entry type alias should have a HIR item");
        let ItemKind::Type { ty, .. } = buffer.kind else {
            panic!("NestedBuffer should be a type alias");
        };
        let TypeKind::Array {
            len: ArrayLen::Expr(len),
            ..
        } = semantics.hir()[ty].kind
        else {
            panic!("NestedBuffer should retain its array-length expression");
        };

        assert!(matches!(
            semantics.eval().value_for_hir_expr(len),
            Some(ConstValue::Int(value)) if value.value == 7
        ));
    }
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
) -> DefId {
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
    name: Str<'cx>,
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

mod phases {
    use super::*;
    use syn_sem_hir as hir;
    use syn_sem_infer::{
        GenericArg, InferConstFacts, InferDb, PrimitiveType, ProjectionNormalizationResult,
        ProjectionType, Type, TypeId,
    };

    // Validates the intended upper-phase consumption pattern:
    // traverse HIR source spine through syn-sem-hir and query definition/scope facts through
    // syn-sem-name, without depending on syn-sem-ast directly.
    #[test]
    fn consumes_hir_and_name_facts_together() {
        // Proves upper phases consume HIR traversal and name facts together.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx.analyze_virtual_file("upper_phase.rs").unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = InferDb::analyze(&tcx.common, hir, names, &InferConstFacts::default());

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
            .map(|param| param.ty)
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

        let local_ty_id = hir[signature].params[0].ty;
        let infer_local_ty_id = infer
            .type_for_hir_type(local_ty_id)
            .expect("signature return type should be lowered");
        assert_eq!(infer.nominal_def(infer_local_ty_id), Some(local_def));

        let usize_ty_id = hir[signature].params[2].ty;
        let infer_usize_ty_id = infer
            .type_for_hir_type(usize_ty_id)
            .expect("signature parameter type should be lowered");
        let Type::Primitive(primitive) = infer[infer_usize_ty_id] else {
            panic!("usize signature parameter should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::Usize);

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

        let t_return_ty_id = hir[generic_signature].params[0].ty;
        let infer_t_return_ty_id = infer
            .type_for_hir_type(t_return_ty_id)
            .expect("generic return type should be lowered");
        assert_eq!(infer.generic_param_def(infer_t_return_ty_id), Some(t_def));
    }

    #[test]
    fn consumes_projection_normalization_query_from_hir() {
        // Proves top-level semantics exposes projection normalization by HIR type.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("projection_normalization.rs")
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = InferDb::analyze(&tcx.common, hir, names, &InferConstFacts::default());

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
        let field_ty_id = hir[*field].ty;
        assert_eq!(hir[field_ty_id].source, hir::TypeSource::StructField);

        let projection = infer
            .type_for_hir_type(field_ty_id)
            .expect("Output.field type should be lowered");
        let ProjectionType {
            assoc,
            self_,
            trait_,
        } = infer
            .projection(projection)
            .expect("Output.field should remain a projection path");
        assert!(self_.is_some());
        assert!(trait_.is_some());
        assert_eq!(names[*assoc].kind, DefKind::AssocType);

        let ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty_id] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn consumes_generic_projection_normalization_query_from_hir() {
        // Proves top-level normalization handles generic projection values from HIR.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("generic_projection_normalization.rs")
            .unwrap();
        let hir = semantics.hir();
        let infer = InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &InferConstFacts::default(),
        );

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
        let field_ty_id = hir[*field].ty;
        let projection = infer
            .type_for_hir_type(field_ty_id)
            .expect("Output.field type should be lowered");
        assert!(infer.projection(projection).is_some());

        let ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty_id] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn normalizes_projection_with_multiple_generic_bindings() {
        // Proves top-level normalization selects the right binding among multiple generics.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("multi_generic_projection_normalization.rs")
            .unwrap();
        let hir = semantics.hir();
        let infer = InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &InferConstFacts::default(),
        );

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
        let field_ty_id = hir[*field].ty;

        let projection = infer
            .type_for_hir_type(field_ty_id)
            .expect("Result.field type should be lowered");
        let ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let Type::Primitive(primitive) = infer[normalized_ty_id] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::Bool);
    }

    #[test]
    fn normalizes_nested_generic_projection_values() {
        // Proves top-level normalization substitutes nested generic projection values.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("nested_generic_projection_normalization.rs")
            .unwrap();
        let hir = semantics.hir();
        let infer = InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &InferConstFacts::default(),
        );

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
        let field_ty_id = hir[*field].ty;

        let projection = infer
            .type_for_hir_type(field_ty_id)
            .expect("Result.field type should be lowered");
        let ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let Type::Path(path) = &infer[normalized_ty_id] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<u32> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArg::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<u32> should have one type argument");
        };
        let Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, PrimitiveType::U32);
    }

    #[test]
    fn consumes_recursive_normalization_query_from_hir() {
        // Proves top-level semantics recursively normalizes projections inside containers.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("recursive_projection_normalization.rs")
            .unwrap();
        let hir = semantics.hir();
        let mut infer = InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &InferConstFacts::default(),
        );

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
        let field_ty_id = hir[*field].ty;

        let normalized_ty_id = infer
            .normalized_type_for_hir_type(field_ty_id)
            .expect("Result.field type should be lowered");
        assert_option_of(&infer, normalized_ty_id, PrimitiveType::U32);
    }

    #[test]
    fn keeps_generic_substitutions_tied_to_impl_self_match() {
        // Proves top-level substitutions stay tied to their impl-self match context.
        let tcx = TopCx::default();
        tcx.add_virtual_file(
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
        let semantics = tcx
            .analyze_virtual_file("contextual_projection_substitution.rs")
            .unwrap();
        let hir = semantics.hir();
        let infer = InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &InferConstFacts::default(),
        );

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

        let vec_field_ty_id = hir[*vec_field].ty;
        let box_field_ty_id = hir[*box_field].ty;

        let vec_projection = infer
            .type_for_hir_type(vec_field_ty_id)
            .expect("Result.vec_field type should be lowered");
        let ProjectionNormalizationResult::Known(vec_normalized_ty_id) =
            infer.projection_normalization(vec_projection)
        else {
            panic!("Vec projection should have one context-matched normalization");
        };
        assert_option_of(&infer, vec_normalized_ty_id, PrimitiveType::U32);

        let box_projection = infer
            .type_for_hir_type(box_field_ty_id)
            .expect("Result.box_field type should be lowered");
        let ProjectionNormalizationResult::Known(box_normalized_ty_id) =
            infer.projection_normalization(box_projection)
        else {
            panic!("Box projection should have one context-matched normalization");
        };
        assert_option_of(&infer, box_normalized_ty_id, PrimitiveType::Bool);
    }

    fn assert_option_of(infer: &InferDb<'_>, ty: TypeId, expected: PrimitiveType) {
        let Type::Path(path) = &infer[ty] else {
            panic!("normalized projection value should lower to path type");
        };
        let [segment] = path.path.segments.as_slice() else {
            panic!("Option<T> should have one path segment");
        };
        assert_eq!(segment.name.as_ref(), "Option");
        let [GenericArg::Type(arg)] = segment.args.as_slice() else {
            panic!("Option<T> should have one type argument");
        };
        let Type::Primitive(primitive) = infer[*arg] else {
            panic!("Option argument should lower to primitive type");
        };
        assert_eq!(primitive, expected);
    }

    #[test]
    fn analyzes_multi_file_virtual_module_tree() {
        let tcx = TopCx::default();
        tcx.add_virtual_file("/virtual/sub.rs", "pub fn helper() -> usize { 42 }")
            .unwrap();
        tcx.add_virtual_file(
            "/virtual/main.rs",
            r#"
            mod sub;
            pub fn main() -> usize {
                sub::helper()
            }
            "#,
        )
        .unwrap();

        let semantics = tcx.analyze_virtual_file("/virtual/main.rs").unwrap();
        let hir = semantics.hir();
        assert_eq!(hir.files().len(), 2);
    }
}
