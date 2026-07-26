use syn_sem_ast::{self as ast, SourceKind};
use syn_sem_common::{CommonCx, SourceText};
use syn_sem_name::{AstNodeId, DefId, DefKind, Name, NameDb, Namespace, ResolveResult, ScopeId};

fn parse_input<'cx>(
    scx: &'cx ast::SyntaxCx<'cx>,
    source_text: SourceText<'cx>,
) -> ast::SourceInput<'cx> {
    let file_path = scx.common.intern("test.rs");
    scx.parse_file(file_path, source_text, SourceKind::Virtual)
        .unwrap();
    ast::SourceInput {
        file_path,
        file: scx.lookup_source(file_path).unwrap().ast(),
    }
}

fn expect_def<'cx>(
    db: &NameDb<'cx>,
    scope: ScopeId,
    namespace: Namespace,
    name: Name<'cx>,
    kind: DefKind,
) -> DefId {
    let result = match namespace {
        Namespace::Type => db.resolve_type_path(scope, [name].into_iter()),
        Namespace::Value => db.resolve_value_path(scope, [name].into_iter()),
        Namespace::Macro | Namespace::Lifetime => {
            panic!("test helper supports only type and value namespaces")
        }
    };
    let ResolveResult::Found(def) = result else {
        panic!("expected {name:?} to resolve in {namespace:?}");
    };
    assert_eq!(db[def].kind, kind);
    def
}

#[test]
fn resolves_function_generics_params_locals_and_local_items_from_ast() {
    // Proves AST collection resolves generics, params, locals, and local items by scope.
    let ccx = CommonCx::default();
    let scx = ast::SyntaxCx::new(&ccx);
    let source_text = ccx.intern(
        r#"
        fn f<T, const N: usize>(x: T) {
            let y = x;
            struct Local<U> {
                value: U,
            }

            let z: Local<T>;
        }
        "#,
    );
    let input = parse_input(&scx, source_text);
    let fn_item = &input.file.items[0];
    let ast::Item::Fn(fn_data) = fn_item else {
        panic!("expected function item");
    };
    let fn_node = AstNodeId::from_ref(fn_item);
    let block_node = AstNodeId::from_ref(&fn_data.block);
    let db = NameDb::build([input], [input.file_path]).unwrap();
    let fn_def = db
        .def_for_ast_node(fn_node)
        .expect("function should have a definition");
    let generic_scope = db
        .def_generic_scope(fn_def)
        .expect("function should have a generic scope");
    let block_scope = db
        .scope_for_ast_node(block_node)
        .expect("function block should have a scope");

    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("Local"),
        DefKind::Struct,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::GenericType,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("N"),
        DefKind::GenericConst,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("x"),
        DefKind::Local,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("y"),
        DefKind::Local,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("z"),
        DefKind::Local,
    );
    expect_def(
        &db,
        generic_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::GenericType,
    );
}

#[test]
fn keeps_type_and_value_namespaces_separate_from_ast() {
    // Proves AST collection keeps same-spelled type and value names in separate namespaces.
    let ccx = CommonCx::default();
    let scx = ast::SyntaxCx::new(&ccx);
    let source_text = ccx.intern(
        r#"
        fn f<T>(T: i32) {
            let x: T = T;
        }
        "#,
    );
    let input = parse_input(&scx, source_text);
    let ast::Item::Fn(fn_data) = &input.file.items[0] else {
        panic!("expected function item");
    };
    let block_node = AstNodeId::from_ref(&fn_data.block);
    let db = NameDb::build([input], [input.file_path]).unwrap();
    let block_scope = db
        .scope_for_ast_node(block_node)
        .expect("function block should have a scope");

    expect_def(
        &db,
        block_scope,
        Namespace::Type,
        scx.common.intern("T"),
        DefKind::GenericType,
    );
    expect_def(
        &db,
        block_scope,
        Namespace::Value,
        scx.common.intern("T"),
        DefKind::Local,
    );
}
