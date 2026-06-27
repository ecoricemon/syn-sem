use super::*;
use crate::{InferDb, PrimitiveType, Type, TypeId};
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
