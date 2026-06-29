use syn_sem_eval::ConstValue;
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

    fn const_value<'tcx>(
        semantics: &syn_sem_top::Semantics<'tcx>,
        name: &str,
    ) -> Option<ConstValue> {
        let item = semantics
            .hir()
            .items()
            .iter()
            .find(|item| matches!(item.name, Some(item_name) if item_name.as_ref() == name))?;
        let def = item.def?;
        assert_eq!(semantics.names()[def].kind, DefKind::Const);
        semantics.eval().value_for_const_def(def)
    }

    fn assert_const_int(
        semantics: &syn_sem_top::Semantics<'_>,
        name: &str,
        expected_value: u128,
        expected_primitive: infer::PrimitiveType,
    ) {
        let Some(ConstValue::Int(value)) = const_value(semantics, name) else {
            panic!("{name} should evaluate to an integer");
        };
        assert_eq!(value.value, expected_value, "{name} value");
        assert_eq!(value.primitive, expected_primitive, "{name} primitive");
    }

    fn assoc_const_arg_for_field<'cx>(
        hir: &'cx hir::Hir<'cx>,
        field: hir::FieldId,
    ) -> &'cx hir::ConstArg<'cx> {
        let hir::TypeKind::Path(field_path) = &hir[hir[field].ty].kind else {
            panic!("field should be a path type");
        };
        let qself = field_path
            .qself
            .as_ref()
            .expect("field should be a qualified projection");
        let hir::TypeKind::Path(self_path) = &hir[qself.self_].kind else {
            panic!("projection self type should be a path");
        };
        let hir::GenericArg::Type(flag_ty) = &self_path.segments[0].args[0] else {
            panic!("Uses first argument should be a type");
        };
        let hir::TypeKind::Path(flag_path) = &hir[*flag_ty].kind else {
            panic!("Flag argument should be a path type");
        };
        let hir::GenericArg::AssocConst { value, .. } = &flag_path.segments[0].args[0] else {
            panic!("Flag argument should carry an associated const equality");
        };
        value
    }

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
        let infer =
            infer::InferDb::analyze(&tcx.common, hir, names, &infer::InferConstFacts::default());

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
        let infer::Type::Primitive(primitive) = infer[infer_usize_ty_id] else {
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

        let t_return_ty_id = hir[generic_signature].params[0].ty;
        let infer_t_return_ty_id = infer
            .type_for_hir_type(t_return_ty_id)
            .expect("generic return type should be lowered");
        assert_eq!(infer.generic_param_def(infer_t_return_ty_id), Some(t_def));
    }

    #[test]
    fn feeds_evaluated_array_lengths_back_into_inference() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "array_len.rs",
                r#"
            fn f(x: usize) {
                let a = [x; 1 + 2];
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let infer = semantics.infer();

        let item = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "f"))
            .expect("function should be represented");
        let ItemKind::Fn { block, .. } = item.kind else {
            panic!("item should be a function");
        };
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("function should contain one local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Repeat { len, .. } = hir[init].kind else {
            panic!("initializer should be a repeat array expression");
        };

        assert_eq!(semantics.eval().array_len_value(len).unwrap(), Some(3));
        let init_ty = infer
            .type_for_hir_expr(init)
            .expect("repeat array type should be inferred");
        let infer::Type::Array { elem, len } = infer[init_ty] else {
            panic!("repeat array should infer to an array type");
        };
        assert_eq!(
            infer[elem],
            infer::Type::Primitive(infer::PrimitiveType::Usize)
        );
        assert_eq!(len, infer::ArrayLen::ConstUsize(3));
    }

    #[test]
    fn feeds_evaluated_const_generic_args_into_projection_matching() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "const_generic_projection.rs",
                r#"
            struct Array<T, const N: usize> {
                field: T,
            }

            trait Trait {
                type Out;
            }

            impl Trait for Array<u8, 3> {
                type Out = u32;
            }

            struct S {
                field: <Array<u8, { 1 + 2 }> as Trait>::Out,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = semantics.infer();

        let output = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "S"))
            .expect("S struct should be represented");
        let ItemKind::Struct { fields, .. } = &output.kind else {
            panic!("S should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("S should have one field");
        };
        let hir::TypeKind::Path(field_path) = &hir[hir[*field].ty].kind else {
            panic!("S.field should be a path type");
        };
        let const_arg_expr = field_path
            .qself
            .as_ref()
            .and_then(|qself| {
                let hir::TypeKind::Path(self_path) = &hir[qself.self_].kind else {
                    return None;
                };
                self_path.segments.first()
            })
            .and_then(|segment| segment.args.get(1))
            .and_then(|arg| {
                let hir::GenericArg::Const(hir::ConstArg::Expr(expr)) = arg else {
                    return None;
                };
                Some(*expr)
            })
            .expect("projection self type should carry a const expression argument");
        assert_eq!(
            semantics.eval().array_len_value(const_arg_expr).unwrap(),
            Some(3)
        );
        let projection = infer
            .type_for_hir_type(hir[*field].ty)
            .expect("S.field type should be lowered");
        let infer::ProjectionType { assoc, .. } = infer
            .projection(projection)
            .expect("S.field should remain a projection path");
        assert_eq!(names[*assoc].kind, DefKind::AssocType);

        let infer::ProjectionNormalizationResult::Known(value_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("const generic projection should normalize");
        };
        let infer::Type::Primitive(primitive) = infer[value_ty] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::U32);
    }

    #[test]
    fn feeds_evaluated_const_path_args_into_projection_matching() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "const_path_projection.rs",
                r#"
            const N: usize = 1 + 2;

            struct Array<T, const N: usize> {
                field: T,
            }

            trait Trait {
                type Out;
            }

            impl Trait for Array<u8, 3> {
                type Out = u32;
            }

            struct S {
                field: <Array<u8, N> as Trait>::Out,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = semantics.infer();

        let const_item = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "N"))
            .expect("N const item should be represented");
        let Some(const_def) = const_item.def else {
            panic!("N should have a definition");
        };
        let ConstValue::Int(value) = semantics
            .eval()
            .value_for_const_def(const_def)
            .expect("N should be evaluated")
        else {
            panic!("N should evaluate to an integer");
        };
        assert_eq!(value.value, 3);

        let output = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "S"))
            .expect("S struct should be represented");
        let ItemKind::Struct { fields, .. } = &output.kind else {
            panic!("S should be represented as a struct item");
        };
        let [field] = fields.as_slice() else {
            panic!("S should have one field");
        };
        let hir::TypeKind::Path(field_path) = &hir[hir[*field].ty].kind else {
            panic!("S.field should be a path type");
        };
        let const_arg = field_path
            .qself
            .as_ref()
            .and_then(|qself| {
                let hir::TypeKind::Path(self_path) = &hir[qself.self_].kind else {
                    return None;
                };
                self_path.segments.first()
            })
            .and_then(|segment| segment.args.get(1))
            .and_then(|arg| {
                let hir::GenericArg::Const(arg) = arg else {
                    return None;
                };
                Some(arg)
            })
            .expect("projection self type should carry a const path argument");
        let Some(ConstValue::Int(value)) = semantics
            .eval()
            .value_for_const_arg(names, const_arg)
            .expect("N const argument should be evaluated")
        else {
            panic!("N const argument should evaluate to an integer");
        };
        assert_eq!(value.value, 3);

        let projection = infer
            .type_for_hir_type(hir[*field].ty)
            .expect("S.field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(value_ty) =
            infer.projection_normalization(projection)
        else {
            panic!("const path projection should normalize");
        };
        let infer::Type::Primitive(primitive) = infer[value_ty] else {
            panic!("normalized projection value should lower to primitive type");
        };
        assert_eq!(primitive, infer::PrimitiveType::U32);
    }

    #[test]
    fn feeds_evaluated_assoc_const_args_into_projection_matching() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "assoc_const_projection.rs",
                r#"
            const NO: bool = false;

            struct Uses<I, T>;

            trait Flag {
                const PANIC: bool;
            }

            trait Identity {
                type Output;
            }

            impl<T> Identity for Uses<Flag<PANIC = false>, T> {
                type Output = T;
            }

            struct Result {
                expr: <Uses<Flag<PANIC = { false }>, u32> as Identity>::Output,
                path: <Uses<Flag<PANIC = NO>, bool> as Identity>::Output,
                different: <Uses<Flag<PANIC = true>, u32> as Identity>::Output,
            }
            "#,
            )
            .unwrap();
        let hir = semantics.hir();
        let names = semantics.names();
        let infer = semantics.infer();

        let result = hir
            .items()
            .iter()
            .find(|item| matches!(item.name, Some(name) if name.as_ref() == "Result"))
            .expect("Result struct should be represented");
        let ItemKind::Struct { fields, .. } = &result.kind else {
            panic!("Result should be represented as a struct item");
        };
        let [expr_field, path_field, different_field] = fields.as_slice() else {
            panic!("Result should have three fields");
        };
        let Some(ConstValue::Bool(expr_value)) = semantics
            .eval()
            .value_for_const_arg(names, assoc_const_arg_for_field(hir, *expr_field))
            .expect("expr associated const arg should evaluate")
        else {
            panic!("expr associated const arg should evaluate to bool");
        };
        let Some(ConstValue::Bool(path_value)) = semantics
            .eval()
            .value_for_const_arg(names, assoc_const_arg_for_field(hir, *path_field))
            .expect("path associated const arg should evaluate")
        else {
            panic!("path associated const arg should evaluate to bool");
        };
        assert!(!expr_value);
        assert!(!path_value);

        let expr_projection = infer
            .type_for_hir_type(hir[*expr_field].ty)
            .expect("expr field should be lowered");
        let path_projection = infer
            .type_for_hir_type(hir[*path_field].ty)
            .expect("path field should be lowered");
        let different = infer
            .type_for_hir_type(hir[*different_field].ty)
            .expect("different field should be lowered");
        let infer::ProjectionNormalizationResult::Known(expr_ty) =
            infer.projection_normalization(expr_projection)
        else {
            panic!("expr associated const projection should normalize");
        };
        let infer::ProjectionNormalizationResult::Known(path_ty) =
            infer.projection_normalization(path_projection)
        else {
            panic!("path associated const projection should normalize");
        };

        let infer::Type::Primitive(expr_primitive) = infer[expr_ty] else {
            panic!("expr field should normalize to a primitive");
        };
        let infer::Type::Primitive(path_primitive) = infer[path_ty] else {
            panic!("path field should normalize to a primitive");
        };
        assert_eq!(expr_primitive, infer::PrimitiveType::U32);
        assert_eq!(path_primitive, infer::PrimitiveType::Bool);
        assert_eq!(
            infer.projection_normalization(different),
            infer::ProjectionNormalizationResult::NoNormalization
        );
    }

    #[test]
    fn evaluates_typed_integer_const_values() {
        let tcx = TopCx::default();
        let semantics = tcx
            .analyze_virtual_file(
                "typed_const_values.rs",
                r#"
            const SUFFIXED: usize = 3usize;
            const CASTED: usize = (1 + 2) as usize;
            const EXPECTED: usize = 1 + 2;
            const TOO_WIDE: u8 = 300u8;
            "#,
            )
            .unwrap();

        assert_const_int(&semantics, "SUFFIXED", 3, infer::PrimitiveType::Usize);
        assert_const_int(&semantics, "CASTED", 3, infer::PrimitiveType::Usize);
        assert_const_int(&semantics, "EXPECTED", 3, infer::PrimitiveType::Usize);
        assert!(
            const_value(&semantics, "TOO_WIDE").is_none(),
            "overflowing suffixed integer literal should stay unknown"
        );
    }

    #[test]
    fn reports_unsupported_closure_during_evaluation() {
        let tcx = TopCx::default();
        let err = tcx
            .analyze_virtual_file(
                "unsupported_closure.rs",
                r#"
            fn main() {
                let _f = |x| x;
            }
            "#,
            )
            .expect_err("closures should be reported as unsupported");
        let message = err.to_string();
        assert!(message.contains("EvalDb::evaluate_expr"));
        assert!(message.contains("unsupported closure expression"));
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
        let infer =
            infer::InferDb::analyze(&tcx.common, hir, names, &infer::InferConstFacts::default());

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
        let infer::ProjectionType {
            assoc,
            self_,
            trait_,
        } = infer
            .projection(projection)
            .expect("Output.field should remain a projection path");
        assert!(self_.is_some());
        assert!(trait_.is_some());
        assert_eq!(names[*assoc].kind, DefKind::AssocType);

        let infer::ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_ty_id] else {
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
        let infer = infer::InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &infer::InferConstFacts::default(),
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

        let infer::ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Output.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_ty_id] else {
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
        let infer = infer::InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &infer::InferConstFacts::default(),
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
        let infer::ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let infer::Type::Primitive(primitive) = infer[normalized_ty_id] else {
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
        let infer = infer::InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &infer::InferConstFacts::default(),
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
        let infer::ProjectionNormalizationResult::Known(normalized_ty_id) =
            infer.projection_normalization(projection)
        else {
            panic!("Result.field projection should have one normalization");
        };
        let infer::Type::Path(path) = &infer[normalized_ty_id] else {
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
        let mut infer = infer::InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &infer::InferConstFacts::default(),
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
        assert_option_of(&infer, normalized_ty_id, infer::PrimitiveType::U32);
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
        let infer = infer::InferDb::analyze(
            &tcx.common,
            hir,
            semantics.names(),
            &infer::InferConstFacts::default(),
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
        let infer::ProjectionNormalizationResult::Known(vec_normalized_ty_id) =
            infer.projection_normalization(vec_projection)
        else {
            panic!("Vec projection should have one context-matched normalization");
        };
        assert_option_of(&infer, vec_normalized_ty_id, infer::PrimitiveType::U32);

        let box_projection = infer
            .type_for_hir_type(box_field_ty_id)
            .expect("Result.box_field type should be lowered");
        let infer::ProjectionNormalizationResult::Known(box_normalized_ty_id) =
            infer.projection_normalization(box_projection)
        else {
            panic!("Box projection should have one context-matched normalization");
        };
        assert_option_of(&infer, box_normalized_ty_id, infer::PrimitiveType::Bool);
    }

    fn assert_option_of(
        infer: &infer::InferDb<'_>,
        ty: infer::TypeId,
        expected: infer::PrimitiveType,
    ) {
        let infer::Type::Path(path) = &infer[ty] else {
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
