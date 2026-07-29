use super::*;
use crate::{ArrayLen, InferConstFacts, InferDb, PrimitiveType, Type, TypeId};
use syn_sem_ast::{SourceInput, SourceKind, SyntaxCx};
use syn_sem_common::{known::KnownLibraryConfig, CommonCx, Str};
use syn_sem_hir as hir;
use syn_sem_hir::{Hir, ItemKind};
use syn_sem_name::{DefKind, NameDb};

fn analyze<'cx>(
    ccx: &'cx CommonCx,
    scx: &'cx SyntaxCx<'cx>,
    source_text: &str,
) -> (NameDb<'cx>, Hir<'cx>, InferDb<'cx>) {
    analyze_with_known(
        ccx,
        scx,
        source_text,
        KnownLibraryConfig {
            core: false,
            std: false,
        },
    )
}

fn analyze_with_known<'cx>(
    ccx: &'cx CommonCx,
    scx: &'cx SyntaxCx<'cx>,
    source_text: &str,
    known: KnownLibraryConfig,
) -> (NameDb<'cx>, Hir<'cx>, InferDb<'cx>) {
    let entry_path = ccx.intern("subject_type_infer_test.rs");
    let entry = parse_source(ccx, scx, entry_path, source_text);
    let mut inputs = vec![entry];
    let mut roots = vec![entry_path];
    for known in known.sources() {
        let file_path = ccx
            .insert_virtual_file(known.path, known.source_text)
            .expect("known source should be stored");
        let source_text = ccx
            .source_text(file_path)
            .expect("known source text should be stored");
        scx.parse_file(file_path, source_text, SourceKind::Known)
            .expect("known source should parse");
        roots.push(file_path);
        inputs.push(parse_stored_source(scx, file_path));
    }

    let names = NameDb::build(inputs.clone(), roots).expect("name collection should succeed");
    let hir = Hir::build(&names, inputs);
    let infer = InferDb::analyze(ccx, &hir, &names, &InferConstFacts::default());
    (names, hir, infer)
}

fn parse_source<'cx>(
    ccx: &'cx CommonCx,
    scx: &'cx SyntaxCx<'cx>,
    file_path: Str<'cx>,
    source_text: &str,
) -> SourceInput<'cx> {
    let source_text = ccx.intern(source_text);
    scx.parse_file(file_path, source_text, SourceKind::Virtual)
        .expect("test input should parse");
    parse_stored_source(scx, file_path)
}

fn parse_stored_source<'cx>(scx: &'cx SyntaxCx<'cx>, file_path: Str<'cx>) -> SourceInput<'cx> {
    let file = scx.lookup_source(file_path).unwrap().ast();
    SourceInput { file_path, file }
}

fn function_block(hir: &Hir<'_>, name: &str) -> hir::BlockId {
    let item = hir
        .items()
        .iter()
        .find(|item| {
            item.name
                .is_some_and(|item_name| item_name.as_ref() == name)
        })
        .unwrap_or_else(|| panic!("expected function `{name}`"));
    let ItemKind::Fn { block, .. } = item.kind else {
        panic!("expected function item");
    };
    block
}

mod relations {
    use super::*;

    #[test]
    fn resolves_simple_type_relations_through_logic_equalities() {
        // Proves collected equality facts resolve local, initializer, and tail expression types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) -> usize {
                let y = x;
                y
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local), hir::lower::Stmt::Expr(tail)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected local statement followed by tail expression");
        };
        let init = local.init.expect("local should have initializer");
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert!(infer.type_relations.equalities().iter().any(|fact| {
            fact.left == TypeSubject::Def(local_def) && fact.right == TypeSubject::Expr(init)
        }));
        assert_usize(&infer, infer.type_for_def(local_def));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        let tail_ty_id = infer.type_for_hir_expr(*tail);
        assert_usize(&infer, tail_ty_id);
        assert!(infer.type_relations.resolved().iter().any(|fact| {
            fact.subject == TypeSubject::Expr(*tail) && Some(fact.ty) == tail_ty_id
        }));
        assert!(matches!(names[local_def].kind, DefKind::Local));
    }
}

mod expressions {
    use super::*;

    #[test]
    fn derives_reference_expression_types_from_resolved_operands() {
        // Proves reference expressions derive `&T` from resolved operand types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) {
                let r = &x;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Reference {
            expr: operand,
            is_mut,
        } = hir[init].kind
        else {
            panic!("expected reference initializer");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert!(!is_mut);
        assert_usize(&infer, infer.type_for_hir_expr(operand));
        assert_reference_to_primitive(
            &infer,
            infer.type_for_hir_expr(init),
            PrimitiveType::Usize,
            false,
        );
        assert_reference_to_primitive(
            &infer,
            infer.type_for_def(local_def),
            PrimitiveType::Usize,
            false,
        );
    }

    #[test]
    fn derives_struct_field_expression_types_from_resolved_base() {
        // Proves field access derives the declared field type from the resolved base struct.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            struct Point {
                x: usize,
                y: bool,
            }

            fn f(point: Point) {
                let x = point.x;
                let y = point.y;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(x_local), hir::lower::Stmt::Local(y_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let x_init = x_local.init.expect("x local should have initializer");
        let y_init = y_local.init.expect("y local should have initializer");
        let hir::ExprKind::Field { base: x_base, .. } = hir[x_init].kind else {
            panic!("expected x field initializer");
        };
        let hir::ExprKind::Field { base: y_base, .. } = hir[y_init].kind else {
            panic!("expected y field initializer");
        };

        assert!(infer.type_for_hir_expr(x_base).is_some());
        assert!(infer.type_for_hir_expr(y_base).is_some());
        assert_usize(&infer, infer.type_for_hir_expr(x_init));
        assert_usize(&infer, type_for_local(&infer, x_local));
        assert_primitive(&infer, infer.type_for_hir_expr(y_init), PrimitiveType::Bool);
        assert_primitive(&infer, type_for_local(&infer, y_local), PrimitiveType::Bool);
    }

    #[test]
    fn derives_tuple_expression_types_from_resolved_operands() {
        // Proves tuple expressions derive element types from resolved operands.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize, y: bool) {
                let t = (x, y);
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Tuple { elems } = &hir[init].kind else {
            panic!("expected tuple initializer");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        let [x_expr, y_expr] = elems.as_slice() else {
            panic!("expected two tuple elements");
        };
        assert_usize(&infer, infer.type_for_hir_expr(*x_expr));
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(*y_expr),
            PrimitiveType::Bool,
        );
        assert_tuple_of_primitives(
            &infer,
            infer.type_for_hir_expr(init),
            &[PrimitiveType::Usize, PrimitiveType::Bool],
        );
        assert_tuple_of_primitives(
            &infer,
            infer.type_for_def(local_def),
            &[PrimitiveType::Usize, PrimitiveType::Bool],
        );
    }

    #[test]
    fn derives_array_expression_types_from_resolved_elements() {
        // Proves array expressions derive element type and literal length from resolved elements.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize, y: usize) {
                let a = [x, y];
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Array { elems } = &hir[init].kind else {
            panic!("expected array initializer");
        };
        let [x_expr, y_expr] = elems.as_slice() else {
            panic!("expected two array elements");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert_usize(&infer, infer.type_for_hir_expr(*x_expr));
        assert_usize(&infer, infer.type_for_hir_expr(*y_expr));
        assert_array_of_primitive(
            &infer,
            infer.type_for_hir_expr(init),
            PrimitiveType::Usize,
            ArrayLen::ConstUsize(2),
        );
        assert_array_of_primitive(
            &infer,
            infer.type_for_def(local_def),
            PrimitiveType::Usize,
            ArrayLen::ConstUsize(2),
        );
    }

    #[test]
    fn derives_repeat_expression_types_from_resolved_operand() {
        // Proves repeat expressions derive array type from value and length operands.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize, n: usize) {
                let a = [x; n];
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Repeat { expr, len } = hir[init].kind else {
            panic!("expected repeat initializer");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert_usize(&infer, infer.type_for_hir_expr(expr));
        assert_usize(&infer, infer.type_for_hir_expr(len));
        assert_array_of_primitive(
            &infer,
            infer.type_for_hir_expr(init),
            PrimitiveType::Usize,
            ArrayLen::Expr(len),
        );
        assert_array_of_primitive(
            &infer,
            infer.type_for_def(local_def),
            PrimitiveType::Usize,
            ArrayLen::Expr(len),
        );
    }

    #[test]
    fn derives_binary_arithmetic_expression_types() {
        // Proves arithmetic binary expressions keep operand and result types aligned.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) {
                let y = x + 1;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Binary { left, right, .. } = hir[init].kind else {
            panic!("expected binary initializer");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert_usize(&infer, infer.type_for_hir_expr(left));
        assert_usize(&infer, infer.type_for_hir_expr(right));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        assert_usize(&infer, infer.type_for_def(local_def));
    }

    #[test]
    fn derives_add_expression_type_from_core_ops_output_projection() {
        // Proves `+` uses the `core::ops::Add::Output` projection path when core facts exist.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir, infer) = analyze_with_known(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) {
                let y = x + 1;
            }
            "#,
            KnownLibraryConfig {
                core: true,
                std: false,
            },
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected local statement");
        };
        let init = local.init.expect("local should have initializer");
        let hir::ExprKind::Binary {
            op: hir::BinaryOp::Add,
            left,
            right,
        } = hir[init].kind
        else {
            panic!("expected add initializer");
        };
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert_usize(&infer, infer.type_for_hir_expr(left));
        assert_usize(&infer, infer.type_for_hir_expr(right));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        assert_usize(&infer, infer.type_for_def(local_def));
        assert!(
            infer
                .projections
                .normalizations
                .iter()
                .any(|normalization| {
                    names[normalization.assoc]
                        .name
                        .is_some_and(|name| name.as_ref() == "Output")
                        && infer[normalization.value_ty] == Type::Primitive(PrimitiveType::Usize)
                }),
            "expected Add::Output projection normalization to produce usize"
        );
    }

    #[test]
    fn derives_arithmetic_expression_type_for_reference_operands_from_core_ops_output_projection() {
        // Proves reference arithmetic operands use the matching `core::ops::*::Output` impl facts.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze_with_known(
            &ccx,
            &scx,
            r#"
            fn f(value: usize, left: &usize, right: &usize) {
                let add_both_ref = left + right;
                let add_left_value = value + right;
                let add_right_value = left + value;
                let sub_both_ref = left - right;
                let sub_left_value = value - right;
                let sub_right_value = left - value;
                let mul_both_ref = left * right;
                let mul_left_value = value * right;
                let mul_right_value = left * value;
                let div_both_ref = left / right;
                let div_left_value = value / right;
                let div_right_value = left / value;
                let rem_both_ref = left % right;
                let rem_left_value = value % right;
                let rem_right_value = left % value;
            }
            "#,
            KnownLibraryConfig {
                core: true,
                std: false,
            },
        );

        let block = function_block(&hir, "f");
        let expected_ops = [
            hir::BinaryOp::Add,
            hir::BinaryOp::Add,
            hir::BinaryOp::Add,
            hir::BinaryOp::Sub,
            hir::BinaryOp::Sub,
            hir::BinaryOp::Sub,
            hir::BinaryOp::Mul,
            hir::BinaryOp::Mul,
            hir::BinaryOp::Mul,
            hir::BinaryOp::Div,
            hir::BinaryOp::Div,
            hir::BinaryOp::Div,
            hir::BinaryOp::Rem,
            hir::BinaryOp::Rem,
            hir::BinaryOp::Rem,
        ];
        let stmts = hir.lowered_blocks()[block].stmts.as_slice();
        assert_eq!(stmts.len(), expected_ops.len());
        for (stmt, expected_op) in stmts.iter().zip(expected_ops) {
            let hir::lower::Stmt::Local(local) = stmt else {
                panic!("expected arithmetic local statement");
            };
            assert_binary_local_is_usize(&hir, &infer, local, expected_op);
        }
    }

    #[test]
    fn derives_bitwise_expression_type_for_reference_operands_from_core_ops_output_projection() {
        // Proves reference bitwise operands use the matching `core::ops::*::Output` impl facts.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze_with_known(
            &ccx,
            &scx,
            r#"
            fn f(value: usize, left: &usize, right: &usize, flag: bool, other_flag: bool) {
                let xor_both_ref = left ^ right;
                let and_left_value = value & right;
                let or_right_value = left | value;
                let bool_xor = flag ^ other_flag;
                let bool_and = flag & other_flag;
                let bool_or = flag | other_flag;
            }
            "#,
            KnownLibraryConfig {
                core: true,
                std: false,
            },
        );

        let block = function_block(&hir, "f");
        let expected = [
            (hir::BinaryOp::BitXor, PrimitiveType::Usize),
            (hir::BinaryOp::BitAnd, PrimitiveType::Usize),
            (hir::BinaryOp::BitOr, PrimitiveType::Usize),
            (hir::BinaryOp::BitXor, PrimitiveType::Bool),
            (hir::BinaryOp::BitAnd, PrimitiveType::Bool),
            (hir::BinaryOp::BitOr, PrimitiveType::Bool),
        ];
        let stmts = hir.lowered_blocks()[block].stmts.as_slice();
        assert_eq!(stmts.len(), expected.len());
        for (stmt, (expected_op, expected_ty)) in stmts.iter().zip(expected) {
            let hir::lower::Stmt::Local(local) = stmt else {
                panic!("expected bitwise local statement");
            };
            assert_binary_local_primitive(&hir, &infer, local, expected_op, expected_ty);
        }
    }

    #[test]
    fn derives_binary_comparison_and_logic_expression_types() {
        // Proves comparison and logical binary expressions resolve to bool results.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize, flag: bool) {
                let same = x == 1;
                let both = flag && true;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(eq_local), hir::lower::Stmt::Local(and_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let eq_init = eq_local
            .init
            .expect("comparison local should have initializer");
        let hir::ExprKind::Binary {
            left: eq_left,
            right: eq_right,
            ..
        } = hir[eq_init].kind
        else {
            panic!("expected comparison initializer");
        };
        let and_init = and_local.init.expect("logic local should have initializer");
        let hir::ExprKind::Binary {
            left: and_left,
            right: and_right,
            ..
        } = hir[and_init].kind
        else {
            panic!("expected logic initializer");
        };

        assert_usize(&infer, infer.type_for_hir_expr(eq_left));
        assert_usize(&infer, infer.type_for_hir_expr(eq_right));
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(eq_init),
            PrimitiveType::Bool,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(and_left),
            PrimitiveType::Bool,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(and_right),
            PrimitiveType::Bool,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(and_init),
            PrimitiveType::Bool,
        );
    }

    #[test]
    fn derives_unary_expression_types() {
        // Proves unary expressions derive negation, boolean-not, and bit-not result types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: i32, flag: bool, mask: usize) {
                let negated = -x;
                let inverted = !flag;
                let bits = !mask;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(neg_local), hir::lower::Stmt::Local(bool_not_local), hir::lower::Stmt::Local(bit_not_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected three local statements");
        };
        let neg_init = neg_local
            .init
            .expect("negation local should have initializer");
        let bool_not_init = bool_not_local
            .init
            .expect("boolean not local should have initializer");
        let bit_not_init = bit_not_local
            .init
            .expect("bitwise not local should have initializer");

        assert_primitive(
            &infer,
            infer.type_for_hir_expr(neg_init),
            PrimitiveType::I32,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(bool_not_init),
            PrimitiveType::Bool,
        );
        assert_usize(&infer, infer.type_for_hir_expr(bit_not_init));
    }

    #[test]
    fn derives_deref_expression_types_from_resolved_references() {
        // Proves dereference expressions derive the referenced element type.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) {
                let r = &x;
                let y = *r;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(ref_local), hir::lower::Stmt::Local(deref_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let ref_init = ref_local
            .init
            .expect("reference local should have initializer");
        let deref_init = deref_local
            .init
            .expect("dereference local should have initializer");
        let hir::ExprKind::Unary {
            op: hir::UnaryOp::Deref,
            expr: operand,
        } = hir[deref_init].kind
        else {
            panic!("expected dereference initializer");
        };

        assert_reference_to_primitive(
            &infer,
            infer.type_for_hir_expr(ref_init),
            PrimitiveType::Usize,
            false,
        );
        assert_reference_to_primitive(
            &infer,
            infer.type_for_hir_expr(operand),
            PrimitiveType::Usize,
            false,
        );
        assert_usize(&infer, infer.type_for_hir_expr(deref_init));
        assert_usize(&infer, type_for_local(&infer, deref_local));
    }
}

mod calls {
    use super::*;

    #[test]
    fn derives_function_call_result_and_argument_types() {
        // Proves function calls propagate parameter and return types to arguments and results.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn takes_usize(x: usize) -> usize {
                x
            }

            fn choose(flag: bool, value: usize) -> usize {
                value
            }

            fn pair(x: usize, y: bool) -> (usize, bool) {
                (x, y)
            }

            fn identity<T>(value: T) -> T {
                value
            }

            fn f(input: usize) {
                let y = takes_usize(1);
                let selected = choose(true, 1);
                let result = pair(1, false);
                let generic = identity(input);
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(single), hir::lower::Stmt::Local(multi), hir::lower::Stmt::Local(pair), hir::lower::Stmt::Local(generic)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected four local statements");
        };
        assert_call_usize(&hir, &infer, single);

        let multi_init = multi.init.expect("multi call should have initializer");
        let hir::ExprKind::Call {
            args: multi_args, ..
        } = &hir[multi_init].kind
        else {
            panic!("expected multi call initializer");
        };
        let [flag_arg, value_arg] = multi_args.as_slice() else {
            panic!("expected two multi call arguments");
        };
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(*flag_arg),
            PrimitiveType::Bool,
        );
        assert_usize(&infer, infer.type_for_hir_expr(*value_arg));
        assert_usize(&infer, infer.type_for_hir_expr(multi_init));
        assert_usize(&infer, type_for_local(&infer, multi));

        let pair_init = pair.init.expect("pair call should have initializer");
        let hir::ExprKind::Call {
            args: pair_args, ..
        } = &hir[pair_init].kind
        else {
            panic!("expected pair call initializer");
        };
        let [x_arg, y_arg] = pair_args.as_slice() else {
            panic!("expected two pair call arguments");
        };
        assert_usize(&infer, infer.type_for_hir_expr(*x_arg));
        assert_primitive(&infer, infer.type_for_hir_expr(*y_arg), PrimitiveType::Bool);
        assert_tuple_of_primitives(
            &infer,
            infer.type_for_hir_expr(pair_init),
            &[PrimitiveType::Usize, PrimitiveType::Bool],
        );
        assert_tuple_of_primitives(
            &infer,
            type_for_local(&infer, pair),
            &[PrimitiveType::Usize, PrimitiveType::Bool],
        );

        assert_call_usize(&hir, &infer, generic);
    }
}

mod patterns {
    use super::*;

    #[test]
    fn derives_pattern_bindings_from_initializer_parameter_and_annotation() {
        // Proves tuple and struct patterns derive binding types from initializers, params, and annotations.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            struct Point {
                x: usize,
                y: bool,
            }

            fn f(pair: (usize, bool), point: Point) {
                let (a, b) = pair;
                let Point { x, y } = point;
                let (typed_a, typed_b): (usize, bool) = (1, true);
            }

            fn g(Point { x: param_x, y: param_y }: Point) {}
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(tuple_local), hir::lower::Stmt::Local(struct_local), hir::lower::Stmt::Local(annotated_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected three local statements");
        };

        assert_two_bindings(
            &infer,
            tuple_local,
            PrimitiveType::Usize,
            PrimitiveType::Bool,
        );
        assert_two_bindings(
            &infer,
            struct_local,
            PrimitiveType::Usize,
            PrimitiveType::Bool,
        );
        assert_two_bindings(
            &infer,
            annotated_local,
            PrimitiveType::Usize,
            PrimitiveType::Bool,
        );

        let item = hir
            .items()
            .iter()
            .find(|item| item.name.is_some_and(|name| name.as_ref() == "g"))
            .expect("expected function item");
        let ItemKind::Fn { signature, .. } = item.kind else {
            panic!("expected function item");
        };
        let param_pat = hir[signature].params[1]
            .pat
            .expect("input parameter should have a pattern");
        let bindings = pat_bindings(&hir, param_pat);
        let [x_def, y_def] = bindings.as_slice() else {
            panic!("struct parameter pattern should introduce two bindings");
        };
        assert_primitive(&infer, infer.type_for_def(*x_def), PrimitiveType::Usize);
        assert_primitive(&infer, infer.type_for_def(*y_def), PrimitiveType::Bool);
    }
}

mod returns {
    use super::*;

    #[test]
    fn constrains_return_operands_to_function_return_types() {
        // Proves return operands are constrained to the surrounding function return type.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(x: usize) -> usize {
                return x;
            }

            fn g() -> i32 {
                return 1;
            }

            fn h(x: usize) -> usize {
                {
                    return x;
                }
            }
            "#,
        );

        let plain = first_return_operand_in_function(&hir, "f");
        let refined = first_return_operand_in_function(&hir, "g");
        let nested = first_return_operand_in_function(&hir, "h");

        assert_usize(&infer, infer.type_for_hir_expr(plain));
        assert_primitive(&infer, infer.type_for_hir_expr(refined), PrimitiveType::I32);
        assert_usize(&infer, infer.type_for_hir_expr(nested));
    }
}

mod numerics {
    use super::*;

    #[test]
    fn keeps_unconstrained_numeric_literals_abstract() {
        // Proves unconstrained integer and float literals remain abstract numeric types.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a = 1;
                let b = 1.0;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(int_local), hir::lower::Stmt::Local(float_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let int_init = int_local.init.expect("int local should have initializer");
        let int_def = int_local
            .bindings
            .first()
            .copied()
            .expect("int local should introduce a binding");
        let float_init = float_local
            .init
            .expect("float local should have initializer");
        let float_def = float_local
            .bindings
            .first()
            .copied()
            .expect("float local should introduce a binding");

        assert_primitive(
            &infer,
            infer.type_for_def(int_def),
            PrimitiveType::AbstractInt,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(int_init),
            PrimitiveType::AbstractInt,
        );
        assert_primitive(
            &infer,
            infer.type_for_def(float_def),
            PrimitiveType::AbstractFloat,
        );
        assert_primitive(
            &infer,
            infer.type_for_hir_expr(float_init),
            PrimitiveType::AbstractFloat,
        );
    }

    #[test]
    fn refines_abstract_numeric_literals_to_concrete_primitives() {
        // Proves annotations and returns refine abstract numeric literals to concrete primitives.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() -> i32 {
                let a: i32 = 1;
                a
            }

            fn g() -> f64 {
                let b: f64 = 1.0;
                b
            }
            "#,
        );

        assert_typed_numeric_local(&hir, &infer, "f", PrimitiveType::I32);
        assert_typed_numeric_local(&hir, &infer, "g", PrimitiveType::F64);
    }

    #[test]
    fn keeps_fresh_abstract_numeric_literals_separate() {
        // Proves refining one numeric literal does not force unrelated fresh literals.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a: i32 = 1;
                let b = 2;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(typed_local), hir::lower::Stmt::Local(untyped_local)] =
            hir.lowered_blocks()[block].stmts.as_slice()
        else {
            panic!("expected two local statements");
        };
        let typed_def = typed_local
            .bindings
            .first()
            .copied()
            .expect("typed local should introduce a binding");
        let untyped_def = untyped_local
            .bindings
            .first()
            .copied()
            .expect("untyped local should introduce a binding");

        assert_primitive(&infer, infer.type_for_def(typed_def), PrimitiveType::I32);
        assert_primitive(
            &infer,
            infer.type_for_def(untyped_def),
            PrimitiveType::AbstractInt,
        );
    }

    #[test]
    fn rejects_incompatible_abstract_numeric_resolution() {
        // Proves incompatible abstract numeric constraints leave the subject unresolved.
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (_names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f() {
                let a: bool = 1;
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
            panic!("expected one local statement");
        };
        let init = local.init.expect("local should have initializer");
        let def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce a binding");

        assert_eq!(infer.type_for_def(def), None);
        assert_eq!(infer.type_for_hir_expr(init), None);
    }
}

fn assert_typed_numeric_local(
    hir: &Hir<'_>,
    infer: &InferDb<'_>,
    name: &str,
    expected: PrimitiveType,
) {
    let block = function_block(hir, name);
    let [hir::lower::Stmt::Local(local), hir::lower::Stmt::Expr(tail)] =
        hir.lowered_blocks()[block].stmts.as_slice()
    else {
        panic!("expected local statement followed by tail expression");
    };
    let init = local.init.expect("local should have initializer");
    let def = local
        .bindings
        .first()
        .copied()
        .expect("local should introduce a binding");

    assert_primitive(infer, infer.type_for_def(def), expected);
    assert_primitive(infer, infer.type_for_hir_expr(init), expected);
    assert_primitive(infer, infer.type_for_hir_expr(*tail), expected);
}

fn assert_usize(infer: &InferDb<'_>, ty: Option<TypeId>) {
    assert_primitive(infer, ty, PrimitiveType::Usize);
}

fn assert_call_usize(hir: &Hir<'_>, infer: &InferDb<'_>, local: &hir::lower::Local) {
    let init = local.init.expect("call local should have initializer");
    let hir::ExprKind::Call { args, .. } = &hir[init].kind else {
        panic!("expected call initializer");
    };
    let [arg] = args.as_slice() else {
        panic!("expected one call argument");
    };

    assert_usize(infer, infer.type_for_hir_expr(*arg));
    assert_usize(infer, infer.type_for_hir_expr(init));
    assert_usize(infer, type_for_local(infer, local));
}

fn assert_binary_local_is_usize(
    hir: &Hir<'_>,
    infer: &InferDb<'_>,
    local: &hir::lower::Local,
    expected_op: hir::BinaryOp,
) {
    assert_binary_local_primitive(hir, infer, local, expected_op, PrimitiveType::Usize);
}

fn assert_binary_local_primitive(
    hir: &Hir<'_>,
    infer: &InferDb<'_>,
    local: &hir::lower::Local,
    expected_op: hir::BinaryOp,
    expected_ty: PrimitiveType,
) {
    let init = local.init.expect("binary local should have initializer");
    let hir::ExprKind::Binary { op, .. } = hir[init].kind else {
        panic!("expected binary initializer");
    };
    assert_eq!(op, expected_op);

    assert_primitive(infer, infer.type_for_hir_expr(init), expected_ty);
    assert_primitive(infer, type_for_local(infer, local), expected_ty);
}

fn type_for_local(infer: &InferDb<'_>, local: &hir::lower::Local) -> Option<TypeId> {
    local
        .bindings
        .first()
        .copied()
        .and_then(|def| infer.type_for_def(def))
}

fn assert_two_bindings(
    infer: &InferDb<'_>,
    local: &hir::lower::Local,
    first: PrimitiveType,
    second: PrimitiveType,
) {
    let [first_def, second_def] = local.bindings.as_slice() else {
        panic!("pattern should introduce two bindings");
    };
    assert_primitive(infer, infer.type_for_def(*first_def), first);
    assert_primitive(infer, infer.type_for_def(*second_def), second);
}

fn assert_primitive(infer: &InferDb<'_>, ty: Option<TypeId>, expected: PrimitiveType) {
    let ty_id = ty.expect("expected a resolved type");
    assert_eq!(infer[ty_id], Type::Primitive(expected));
}

fn first_return_operand_in_function(hir: &Hir<'_>, name: &str) -> hir::ExprId {
    first_return_operand_in_block(hir, function_block(hir, name))
        .unwrap_or_else(|| panic!("expected function `{name}` to contain a value return"))
}

fn first_return_operand_in_block(hir: &Hir<'_>, block: hir::BlockId) -> Option<hir::ExprId> {
    for stmt in &hir.lowered_blocks()[block].stmts {
        let expr = match stmt {
            hir::lower::Stmt::Expr(expr) => *expr,
            hir::lower::Stmt::Local(local) => {
                if let Some(expr) = local.init {
                    expr
                } else {
                    continue;
                }
            }
            hir::lower::Stmt::Item(_) => continue,
        };
        if let Some(operand) = first_return_operand_in_expr(hir, expr) {
            return Some(operand);
        }
    }
    None
}

fn first_return_operand_in_expr(hir: &Hir<'_>, expr: hir::ExprId) -> Option<hir::ExprId> {
    match &hir[expr].kind {
        hir::ExprKind::Return { expr } => *expr,
        hir::ExprKind::Block { block } => first_return_operand_in_block(hir, *block),
        _ => None,
    }
}

fn pat_bindings(hir: &Hir<'_>, pat: hir::PatId) -> Vec<syn_sem_name::DefId> {
    let mut bindings = Vec::new();
    collect_pat_bindings(hir, pat, &mut bindings);
    bindings
}

fn collect_pat_bindings(hir: &Hir<'_>, pat: hir::PatId, bindings: &mut Vec<syn_sem_name::DefId>) {
    match &hir[pat].kind {
        hir::PatKind::Ident { def: Some(def), .. } => bindings.push(*def),
        hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
            collect_pat_bindings(hir, *pat, bindings);
        }
        hir::PatKind::Struct { fields, .. } => {
            for field in fields {
                collect_pat_bindings(hir, field.pat, bindings);
            }
        }
        hir::PatKind::Tuple { elems } => {
            for elem in elems {
                collect_pat_bindings(hir, *elem, bindings);
            }
        }
        hir::PatKind::Ident { def: None, .. }
        | hir::PatKind::Path(_)
        | hir::PatKind::Unsupported => {}
    }
}

fn assert_reference_to_primitive(
    infer: &InferDb<'_>,
    ty: Option<TypeId>,
    expected: PrimitiveType,
    expected_mut: bool,
) {
    let ty_id = ty.expect("expected a resolved type");
    let Type::Reference { elem, is_mut } = infer[ty_id] else {
        panic!("expected reference type");
    };
    assert_eq!(is_mut, expected_mut);
    assert_eq!(infer[elem], Type::Primitive(expected));
}

fn assert_array_of_primitive(
    infer: &InferDb<'_>,
    ty: Option<TypeId>,
    expected: PrimitiveType,
    expected_len: ArrayLen,
) {
    let ty_id = ty.expect("expected a resolved type");
    let Type::Array { elem, len } = infer[ty_id] else {
        panic!("expected array type");
    };
    assert_eq!(infer[elem], Type::Primitive(expected));
    assert_eq!(len, expected_len);
}

fn assert_tuple_of_primitives(infer: &InferDb<'_>, ty: Option<TypeId>, expected: &[PrimitiveType]) {
    let ty_id = ty.expect("expected a resolved type");
    let Type::Tuple { elems } = &infer[ty_id] else {
        panic!("expected tuple type");
    };
    assert_eq!(elems.len(), expected.len());
    for (elem, expected) in elems.iter().zip(expected) {
        assert_eq!(infer[*elem], Type::Primitive(*expected));
    }
}
