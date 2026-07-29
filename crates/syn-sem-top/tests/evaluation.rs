use syn_sem_eval::ConstValue;
use syn_sem_hir::{self as hir, ConstArg, ExprKind, FieldId, GenericArg, Hir, ItemKind, TypeKind};
use syn_sem_infer::{
    self as infer, PrimitiveType, ProjectionNormalizationResult, ProjectionType, Type,
};
use syn_sem_name::DefKind;
use syn_sem_top::{Semantics, TopCx};

fn const_def<'tcx>(semantics: &Semantics<'tcx>, name: &str) -> syn_sem_name::DefId {
    let item = semantics
        .hir()
        .items()
        .iter()
        .find(|item| matches!(item.name, Some(item_name) if item_name.as_ref() == name))
        .unwrap_or_else(|| panic!("{name} should be represented as an item"));
    let def = item
        .def
        .unwrap_or_else(|| panic!("{name} should have a definition"));
    assert_eq!(semantics.names()[def].kind, DefKind::Const);
    def
}

fn const_value<'tcx>(semantics: &Semantics<'tcx>, name: &str) -> Option<ConstValue> {
    semantics
        .eval()
        .value_for_const_def(const_def(semantics, name))
}

fn assert_const_int(
    semantics: &Semantics<'_>,
    name: &str,
    expected_value: u128,
    expected_primitive: PrimitiveType,
) {
    let Some(ConstValue::Int(value)) = const_value(semantics, name) else {
        panic!("{name} should evaluate to an integer");
    };
    assert_eq!(value.value, expected_value, "{name} value");
    assert_eq!(value.primitive, expected_primitive, "{name} primitive");
}

fn assoc_const_arg_for_field<'cx>(hir: &'cx Hir<'cx>, field: FieldId) -> &'cx ConstArg<'cx> {
    let TypeKind::Path(field_path) = &hir[hir[field].ty].kind else {
        panic!("field should be a path type");
    };
    let qself = field_path
        .qself
        .as_ref()
        .expect("field should be a qualified projection");
    let TypeKind::Path(self_path) = &hir[qself.self_].kind else {
        panic!("projection self type should be a path");
    };
    let GenericArg::Type(flag_ty) = &self_path.segments[0].args[0] else {
        panic!("Uses first argument should be a type");
    };
    let TypeKind::Path(flag_path) = &hir[*flag_ty].kind else {
        panic!("Flag argument should be a path type");
    };
    let GenericArg::AssocConst { value, .. } = &flag_path.segments[0].args[0] else {
        panic!("Flag argument should carry an associated const equality");
    };
    value
}

#[test]
fn evaluates_forward_chains_shared_dependencies_and_const_blocks() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "known_values.rs",
            r#"
            const BASE: usize = 2;
            const LEFT: usize = BASE + 1;
            const RIGHT: usize = BASE * 3;
            const FORWARD: usize = LATER + 1;
            const LATER: usize = 4;
            const BLOCK: usize = const { LEFT + RIGHT };
            const BOOL: bool = !false;
            const SUFFIXED: usize = 3usize;
            const CASTED: usize = (1 + 2) as usize;
            type Bytes = [u8; BLOCK];
            "#,
        )
        .unwrap();

    assert_const_int(&semantics, "BASE", 2, PrimitiveType::Usize);
    assert_const_int(&semantics, "LEFT", 3, PrimitiveType::Usize);
    assert_const_int(&semantics, "RIGHT", 6, PrimitiveType::Usize);
    assert_const_int(&semantics, "FORWARD", 5, PrimitiveType::Usize);
    assert_const_int(&semantics, "LATER", 4, PrimitiveType::Usize);
    assert_const_int(&semantics, "BLOCK", 9, PrimitiveType::Usize);
    assert_const_int(&semantics, "SUFFIXED", 3, PrimitiveType::Usize);
    assert_const_int(&semantics, "CASTED", 3, PrimitiveType::Usize);
    assert_eq!(
        const_value(&semantics, "BOOL"),
        Some(ConstValue::Bool(true))
    );
}

#[test]
fn keeps_cyclic_constants_and_their_dependents_unknown() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "cyclic_values.rs",
            r#"
            const DIRECT: usize = DIRECT;
            const LEFT: usize = RIGHT;
            const RIGHT: usize = LEFT;
            const DOWNSTREAM: usize = LEFT + 1;
            const KNOWN: usize = 3;
            "#,
        )
        .expect("constant cycles should not make evaluation fail");

    assert_eq!(const_value(&semantics, "DIRECT"), None);
    assert_eq!(const_value(&semantics, "LEFT"), None);
    assert_eq!(const_value(&semantics, "RIGHT"), None);
    assert_eq!(const_value(&semantics, "DOWNSTREAM"), None);
    assert_const_int(&semantics, "KNOWN", 3, PrimitiveType::Usize);
}

#[test]
fn keeps_failed_const_arithmetic_unknown_through_top_level_analysis() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "unknown_arithmetic.rs",
            r#"
            const DIV_ZERO: usize = 1 / 0;
            const ADD_OVERFLOW: u128 = 340282366920938463463374607431768211455u128 + 1u128;
            const TOO_WIDE: u8 = 300u8;
            "#,
        )
        .expect("arithmetic failure should produce unknown values");

    assert_eq!(const_value(&semantics, "DIV_ZERO"), None);
    assert_eq!(const_value(&semantics, "ADD_OVERFLOW"), None);
    assert_eq!(const_value(&semantics, "TOO_WIDE"), None);
}

#[test]
fn keeps_unresolved_generic_const_values_unknown() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "generic_const.rs",
            r#"
            struct Array<const N: usize> {
                bytes: [u8; N],
            }
            "#,
        )
        .expect("an unresolved generic constant should not make evaluation fail");
    let hir = semantics.hir();
    let len = hir
        .types()
        .iter()
        .find_map(|ty| match ty.kind {
            TypeKind::Array {
                len: hir::ArrayLen::Expr(expr),
                ..
            } => Some(expr),
            _ => None,
        })
        .expect("Array.bytes should contain an array length");

    assert_eq!(semantics.eval().value_for_hir_expr(len), None);
}

#[test]
fn ignores_unsupported_runtime_expressions_but_rejects_const_targets() {
    let tcx = TopCx::default();
    tcx.analyze_virtual_file(
        "runtime_only.rs",
        r#"
        fn runtime() {
            let _closure = || true;
            let _same = 1 == 1;
        }
        "#,
    )
    .expect("unsupported runtime-only expressions should not be evaluated");

    let tcx = TopCx::default();
    let err = tcx
        .analyze_virtual_file(
            "unsupported_constant.rs",
            r#"
            const SAME: bool = 1 == 1;
            "#,
        )
        .expect_err("an unsupported operation in a constant target should fail");
    assert!(err.to_string().contains("unsupported binary op Eq"));

    let tcx = TopCx::default();
    let err = tcx
        .analyze_virtual_file(
            "unsupported_const_closure.rs",
            r#"
            const CLOSURE: bool = || true;
            "#,
        )
        .expect_err("an unsupported expression in a constant target should fail");
    assert!(err.to_string().contains("unsupported expression Closure"));
}

#[test]
fn evaluates_all_required_const_expression_contexts() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "required_contexts.rs",
            r#"
            struct Array<const N: usize> {
                bytes: [u8; N],
            }

            struct Holder<T> {
                value: T,
            }

            trait Trait<const N: usize> {
                type Out;
            }

            trait Flag {
                const VALUE: usize;
            }

            trait Container {
                type Item;
            }

            fn generic<const N: usize>() {}

            fn constrained<T>()
            where
                T: Trait<{ 1 + 2 }>,
            {}

            type Alias = Array<{ 3 + 4 }>;
            type Assoc = Holder<Flag<VALUE = { 13 + 14 }>>;

            fn constrained_assoc<T>()
            where
                T: Container<Item: Trait<{ 15 + 16 }>>,
            {}

            fn targets(value: Array<1>) {
                let _repeat = [0; 5 + 6];
                let _block = const { 7 + 8 };
                generic::<{ 9 + 10 }>();
                value.method::<{ 11 + 12 }>();
            }
            "#,
        )
        .unwrap();
    let hir = semantics.hir();

    let mut const_arg_blocks = 0;
    let mut const_blocks = 0;
    let mut repeat_lengths = 0;
    for expr in hir.exprs() {
        match expr.kind {
            ExprKind::Repeat { len, .. } => {
                let Some(ConstValue::Int(value)) = semantics.eval().value_for_hir_expr(len) else {
                    panic!("repeat length should be evaluated");
                };
                assert_eq!(value.value, 11);
                repeat_lengths += 1;
            }
            ExprKind::Block { .. } => {
                if semantics.eval().value_for_hir_expr(expr.id).is_some() {
                    const_arg_blocks += 1;
                }
            }
            ExprKind::Const { .. } => {
                let Some(ConstValue::Int(value)) = semantics.eval().value_for_hir_expr(expr.id)
                else {
                    panic!("const block should be evaluated");
                };
                assert_eq!(value.value, 15);
                const_blocks += 1;
            }
            _ => {}
        }
    }

    assert_eq!(const_arg_blocks, 6);
    assert_eq!(const_blocks, 1);
    assert_eq!(repeat_lengths, 1);
}

#[test]
fn feeds_evaluated_array_lengths_into_inference() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "array_length_fixed_point.rs",
            r#"
            fn arrays(value: usize) {
                let plain = [value; 1 + 2];
                let blocked = [value; const { 2 + 3 }];
                let parenthesized = [value; (3 + 4)];
            }
            "#,
        )
        .unwrap();
    let hir = semantics.hir();
    let infer = semantics.infer();
    let mut lengths = Vec::new();
    let mut saw_const_block = false;
    let mut saw_parenthesized = false;

    for expr in hir.exprs() {
        let ExprKind::Repeat { len, .. } = expr.kind else {
            continue;
        };
        let evaluated_len = semantics
            .eval()
            .array_len_value(len)
            .unwrap()
            .expect("repeat length should be evaluated");
        lengths.push(evaluated_len);
        saw_const_block |= matches!(hir[len].kind, ExprKind::Const { .. });
        saw_parenthesized |= matches!(hir[len].kind, ExprKind::Paren { .. });

        let ty = infer
            .type_for_hir_expr(expr.id)
            .expect("repeat expression should have an inferred type");
        let Type::Array {
            elem,
            len: inferred_len,
        } = infer[ty]
        else {
            panic!("repeat expression should infer to an array type");
        };
        assert_eq!(infer[elem], Type::Primitive(PrimitiveType::Usize));
        assert_eq!(inferred_len, infer::ArrayLen::ConstUsize(evaluated_len));
    }

    lengths.sort_unstable();
    assert_eq!(lengths, [3, 5, 7]);
    assert!(saw_const_block);
    assert!(saw_parenthesized);
}

#[test]
fn uses_evaluated_const_expression_args_for_projection_matching() {
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
    let TypeKind::Path(field_path) = &hir[hir[*field].ty].kind else {
        panic!("S.field should be a path type");
    };
    let const_arg_expr = field_path
        .qself
        .as_ref()
        .and_then(|qself| {
            let TypeKind::Path(self_path) = &hir[qself.self_].kind else {
                return None;
            };
            self_path.segments.first()
        })
        .and_then(|segment| segment.args.get(1))
        .and_then(|arg| {
            let GenericArg::Const(ConstArg::Expr(expr)) = arg else {
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
    let ProjectionType { assoc, .. } = infer
        .projection(projection)
        .expect("S.field should remain a projection path");
    assert_eq!(names[*assoc].kind, DefKind::AssocType);

    let ProjectionNormalizationResult::Known(value_ty) = infer.projection_normalization(projection)
    else {
        panic!("const generic projection should normalize");
    };
    let Type::Primitive(primitive) = infer[value_ty] else {
        panic!("normalized projection value should lower to primitive type");
    };
    assert_eq!(primitive, PrimitiveType::U32);
}

#[test]
fn uses_evaluated_const_path_args_for_projection_matching() {
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

    assert_const_int(&semantics, "N", 3, PrimitiveType::Usize);

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
    let TypeKind::Path(field_path) = &hir[hir[*field].ty].kind else {
        panic!("S.field should be a path type");
    };
    let const_arg = field_path
        .qself
        .as_ref()
        .and_then(|qself| {
            let TypeKind::Path(self_path) = &hir[qself.self_].kind else {
                return None;
            };
            self_path.segments.first()
        })
        .and_then(|segment| segment.args.get(1))
        .and_then(|arg| {
            let GenericArg::Const(arg) = arg else {
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
    let ProjectionNormalizationResult::Known(value_ty) = infer.projection_normalization(projection)
    else {
        panic!("const path projection should normalize");
    };
    let Type::Primitive(primitive) = infer[value_ty] else {
        panic!("normalized projection value should lower to primitive type");
    };
    assert_eq!(primitive, PrimitiveType::U32);
}

#[test]
fn uses_evaluated_associated_const_args_for_projection_matching() {
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
    let ProjectionNormalizationResult::Known(expr_ty) =
        infer.projection_normalization(expr_projection)
    else {
        panic!("expr associated const projection should normalize");
    };
    let ProjectionNormalizationResult::Known(path_ty) =
        infer.projection_normalization(path_projection)
    else {
        panic!("path associated const projection should normalize");
    };

    let Type::Primitive(expr_primitive) = infer[expr_ty] else {
        panic!("expr field should normalize to a primitive");
    };
    let Type::Primitive(path_primitive) = infer[path_ty] else {
        panic!("path field should normalize to a primitive");
    };
    assert_eq!(expr_primitive, PrimitiveType::U32);
    assert_eq!(path_primitive, PrimitiveType::Bool);
    assert_eq!(
        infer.projection_normalization(different),
        ProjectionNormalizationResult::NoNormalization
    );
}

#[test]
fn evaluates_const_paths_through_renamed_reexports() {
    let tcx = TopCx::default();
    let semantics = tcx
        .analyze_virtual_file(
            "imported_const.rs",
            r#"
            mod values {
                pub const BASE: usize = 2;
            }

            mod middle {
                pub use crate::values::BASE as REEXPORTED;
            }

            use middle::REEXPORTED as RENAMED;
            const RESULT: usize = RENAMED + 1;
            "#,
        )
        .unwrap();

    assert_const_int(&semantics, "BASE", 2, PrimitiveType::Usize);
    assert_const_int(&semantics, "RESULT", 3, PrimitiveType::Usize);
}
