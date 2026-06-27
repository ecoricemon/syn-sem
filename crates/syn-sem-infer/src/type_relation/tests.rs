use super::*;
use crate::{ArrayLen, InferDb, PrimitiveType, Type, TypeId};
use syn_sem_ast as ast;
use syn_sem_ast::SyntaxCx;
use syn_sem_common::CommonCx;
use syn_sem_hir::{Hir, HirBuilder, ItemKind};
use syn_sem_name::{collect::NameCollector, DefKind, NameDb};

fn analyze<'cx>(
    ccx: &'cx CommonCx,
    scx: &'cx SyntaxCx<'cx>,
    source_text: &str,
) -> (NameDb<'cx>, Hir<'cx>, InferDb<'cx>) {
    let file_path = ccx.intern("subject_type_infer_test.rs");
    let source_text = ccx.intern(source_text);
    scx.parse_virtual_file(file_path, source_text)
        .expect("test input should parse");
    let file = scx.lookup_source(file_path).unwrap().ast();
    let names = NameCollector::new([ast::SourceInput { file_path, file }])
        .collect(file_path)
        .expect("name collection should succeed");
    let hir = HirBuilder::new(&names).build(file_path, file);
    let infer = InferDb::analyze(ccx, &hir, &names);
    (names, hir, infer)
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

#[test]
fn resolves_simple_type_relations_through_logic_equalities() {
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
    assert!(infer
        .type_relations
        .resolved()
        .iter()
        .any(|fact| { fact.subject == TypeSubject::Expr(*tail) && Some(fact.ty) == tail_ty_id }));
    assert!(matches!(names[local_def].kind, DefKind::Local));
}

#[test]
fn derives_reference_expression_types_from_resolved_operands() {
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
fn derives_tuple_expression_types_from_resolved_operands() {
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
fn derives_free_function_call_result_and_argument_types() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn takes_usize(x: usize) -> usize {
            x
        }

        fn f() {
            let y = takes_usize(1);
        }
        "#,
    );

    let block = function_block(&hir, "f");
    let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
        panic!("expected local statement");
    };
    let init = local.init.expect("local should have initializer");
    let hir::ExprKind::Call { args, .. } = &hir[init].kind else {
        panic!("expected call initializer");
    };
    let [arg] = args.as_slice() else {
        panic!("expected one call argument");
    };
    let local_def = local
        .bindings
        .first()
        .copied()
        .expect("local should introduce one binding");

    assert_usize(&infer, infer.type_for_hir_expr(*arg));
    assert_usize(&infer, infer.type_for_hir_expr(init));
    assert_usize(&infer, infer.type_for_def(local_def));
}

#[test]
fn derives_tuple_pattern_bindings_from_resolved_initializer() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn f(pair: (usize, bool)) {
            let (a, b) = pair;
        }
        "#,
    );

    let block = function_block(&hir, "f");
    let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
        panic!("expected local statement");
    };
    let [a_def, b_def] = local.bindings.as_slice() else {
        panic!("tuple pattern should introduce two bindings");
    };

    assert_primitive(&infer, infer.type_for_def(*a_def), PrimitiveType::Usize);
    assert_primitive(&infer, infer.type_for_def(*b_def), PrimitiveType::Bool);
}

#[test]
fn derives_tuple_pattern_bindings_from_annotation() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn f() {
            let (a, b): (usize, bool) = (1, true);
        }
        "#,
    );

    let block = function_block(&hir, "f");
    let [hir::lower::Stmt::Local(local)] = hir.lowered_blocks()[block].stmts.as_slice() else {
        panic!("expected local statement");
    };
    let [a_def, b_def] = local.bindings.as_slice() else {
        panic!("tuple pattern should introduce two bindings");
    };

    assert_primitive(&infer, infer.type_for_def(*a_def), PrimitiveType::Usize);
    assert_primitive(&infer, infer.type_for_def(*b_def), PrimitiveType::Bool);
}

#[test]
fn constrains_return_operand_to_function_return_type() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn f(x: usize) -> usize {
            return x;
        }
        "#,
    );

    let operand = first_return_operand_in_function(&hir, "f");

    assert_usize(&infer, infer.type_for_hir_expr(operand));
}

#[test]
fn refines_returned_abstract_numeric_literal_to_return_type() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn f() -> i32 {
            return 1;
        }
        "#,
    );

    let operand = first_return_operand_in_function(&hir, "f");

    assert_primitive(&infer, infer.type_for_hir_expr(operand), PrimitiveType::I32);
}

#[test]
fn constrains_return_operand_inside_nested_blocks() {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let (_names, hir, infer) = analyze(
        &ccx,
        &scx,
        r#"
        fn f(x: usize) -> usize {
            {
                return x;
            }
        }
        "#,
    );

    let operand = first_return_operand_in_function(&hir, "f");

    assert_usize(&infer, infer.type_for_hir_expr(operand));
}

#[test]
fn keeps_unconstrained_numeric_literals_abstract() {
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
