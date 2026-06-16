//! Body and block inference.
//!
//! This module will own block-level orchestration: function bodies, statements, local bindings,
//! tail expressions, and the body-local type environment used while delegating expression typing
//! to `expr`.

use crate::{
    BodyBlockFact, BodyLocalFact, InferDb, PrimitiveType, Type, TypeEqualFact, TypeId, TypeSubject,
};
use syn_sem_hir as hir;
use syn_sem_name::{NameDb, ResolveResult};

pub(crate) fn lower_bodies<'cx>(hir: &hir::Hir<'cx>, names: &NameDb<'cx>, db: &mut InferDb<'cx>) {
    lower_signature_facts(hir, db);
    lower_expr_facts(hir, names, db);
    for block in hir.body().blocks() {
        db.body_block_facts.push(BodyBlockFact {
            block: block.block,
            tail_expr: block.tail_expr,
        });

        for stmt in &block.stmts {
            let hir::lower::Stmt::Local(local) = stmt else {
                continue;
            };
            db.body_local_facts.push(BodyLocalFact {
                block: block.block,
                local: local.local,
                bindings: local.bindings.clone(),
                init: local.init,
            });
            lower_local_facts(hir, db, local);
        }
    }
}

fn lower_signature_facts<'cx>(hir: &hir::Hir<'cx>, db: &mut InferDb<'cx>) {
    for signature in hir.signatures() {
        for param in signature.params.iter().skip(1) {
            lower_pat_type_facts(hir, db, param.pat, param.ty);
        }
    }
}

fn lower_local_facts<'cx>(hir: &hir::Hir<'cx>, db: &mut InferDb<'cx>, local: &hir::lower::Local) {
    let source_local = &hir[local.local];
    if let Some(init) = local.init {
        lower_pat_expr_facts(hir, db, Some(source_local.pat), init);
    }
}

fn lower_expr_facts<'cx>(hir: &hir::Hir<'cx>, names: &NameDb<'cx>, db: &mut InferDb<'cx>) {
    for expr in hir.exprs() {
        match &expr.kind {
            hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                if let Some(tail_expr) = hir.body()[*block].tail_expr {
                    push_type_equal(db, TypeSubject::Expr(expr.id), TypeSubject::Expr(tail_expr));
                }
            }
            hir::ExprKind::Cast { ty, .. } => {
                push_type_equal(
                    db,
                    TypeSubject::Expr(expr.id),
                    TypeSubject::Type(
                        db.type_for_hir_type(*ty)
                            .expect("HIR types are lowered before body facts"),
                    ),
                );
            }
            hir::ExprKind::Lit(lit) => {
                if let Some(ty) = lit_type(db, lit) {
                    push_type_equal(db, TypeSubject::Expr(expr.id), TypeSubject::Type(ty));
                }
            }
            hir::ExprKind::Paren { expr: inner } => {
                push_type_equal(db, TypeSubject::Expr(expr.id), TypeSubject::Expr(*inner));
            }
            hir::ExprKind::Path(path) => {
                if let Some(def) = resolve_value_path(names, expr.scope, &path.segments) {
                    push_type_equal(db, TypeSubject::Expr(expr.id), TypeSubject::Def(def));
                }
            }
            hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Binary { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Reference { .. }
            | hir::ExprKind::Repeat { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. }
            | hir::ExprKind::Tuple { .. }
            | hir::ExprKind::Unary { .. } => {}
        }
    }

    for item in hir.items() {
        let hir::ItemKind::Fn {
            signature, block, ..
        } = item.kind
        else {
            continue;
        };
        let Some(tail_expr) = hir.body()[block].tail_expr else {
            continue;
        };
        let Some(return_param) = hir[signature].params.first() else {
            continue;
        };
        let Some(return_ty) = db.type_for_hir_type(return_param.ty) else {
            continue;
        };
        push_type_equal(
            db,
            TypeSubject::Expr(tail_expr),
            TypeSubject::Type(return_ty),
        );
    }
}

fn lower_pat_type_facts<'cx>(
    hir: &hir::Hir<'cx>,
    db: &mut InferDb<'cx>,
    pat: Option<hir::PatId>,
    ty: hir::TypeId,
) {
    let Some(pat) = pat else {
        return;
    };
    let Some(ty) = db.type_for_hir_type(ty) else {
        return;
    };
    bind_pat_to_type(hir, db, pat, ty);
}

fn bind_pat_to_type<'cx>(hir: &hir::Hir<'cx>, db: &mut InferDb<'cx>, pat: hir::PatId, ty: TypeId) {
    match &hir[pat].kind {
        hir::PatKind::Ident { def: Some(def), .. } => {
            push_type_equal(db, TypeSubject::Def(*def), TypeSubject::Type(ty));
        }
        hir::PatKind::Reference { pat, .. } | hir::PatKind::Type { pat, .. } => {
            bind_pat_to_type(hir, db, *pat, ty);
        }
        hir::PatKind::Ident { def: None, .. }
        | hir::PatKind::Path(_)
        | hir::PatKind::Struct { .. }
        | hir::PatKind::Tuple { .. }
        | hir::PatKind::Unsupported => {}
    }
}

fn lower_pat_expr_facts<'cx>(
    hir: &hir::Hir<'cx>,
    db: &mut InferDb<'cx>,
    pat: Option<hir::PatId>,
    expr: hir::ExprId,
) {
    let Some(pat) = pat else {
        return;
    };
    bind_pat_to_expr(hir, db, pat, expr);
}

fn bind_pat_to_expr<'cx>(
    hir: &hir::Hir<'cx>,
    db: &mut InferDb<'cx>,
    pat: hir::PatId,
    expr: hir::ExprId,
) {
    match &hir[pat].kind {
        hir::PatKind::Ident { def: Some(def), .. } => {
            push_type_equal(db, TypeSubject::Def(*def), TypeSubject::Expr(expr));
        }
        hir::PatKind::Reference { pat, .. } => bind_pat_to_expr(hir, db, *pat, expr),
        hir::PatKind::Type { pat, ty } => {
            let Some(ty) = db.type_for_hir_type(*ty) else {
                return;
            };
            bind_pat_to_type(hir, db, *pat, ty);
            push_type_equal(db, TypeSubject::Expr(expr), TypeSubject::Type(ty));
        }
        hir::PatKind::Ident { def: None, .. }
        | hir::PatKind::Path(_)
        | hir::PatKind::Struct { .. }
        | hir::PatKind::Tuple { .. }
        | hir::PatKind::Unsupported => {}
    }
}

fn resolve_value_path<'cx>(
    names: &NameDb<'cx>,
    scope: Option<syn_sem_name::ScopeId>,
    path: &[hir::PathSegment<'cx>],
) -> Option<syn_sem_name::DefId> {
    let scope = scope?;
    match names.resolve_value_path(scope, path.iter().map(|segment| segment.name)) {
        ResolveResult::Found(def) => Some(def),
        ResolveResult::Ambiguous(_) | ResolveResult::NotFound => None,
    }
}

fn lit_type<'cx>(db: &mut InferDb<'cx>, lit: &hir::Lit<'cx>) -> Option<TypeId> {
    match lit {
        hir::Lit::Bool(_) => Some(db.intern_type(Type::Primitive(PrimitiveType::Bool))),
        hir::Lit::Int(_) | hir::Lit::Float(_) => None,
    }
}

fn push_type_equal(db: &mut InferDb<'_>, left: TypeSubject, right: TypeSubject) {
    let fact = TypeEqualFact { left, right };
    if !db.type_equal_facts.contains(&fact) {
        db.type_equal_facts.push(fact);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let file_path = ccx.intern("body_infer_test.rs");
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
    fn consumes_hir_lowered_body_facts() {
        let ccx = CommonCx::default();
        let scx = SyntaxCx::new(&ccx);
        let (names, hir, infer) = analyze(
            &ccx,
            &scx,
            r#"
            fn f(pair: (usize, usize)) -> usize {
                let (a, b) = pair;
                a
            }
            "#,
        );

        let block = function_block(&hir, "f");
        let block_fact = infer
            .body_block_facts()
            .iter()
            .find(|fact| fact.block == block)
            .expect("expected function block fact");
        assert_eq!(block_fact.tail_expr, hir.body()[block].tail_expr);

        let local_fact = infer
            .body_local_facts()
            .iter()
            .find(|fact| fact.block == block)
            .expect("expected local fact");
        assert_eq!(local_fact.bindings.len(), 2);
        assert!(local_fact.init.is_some());
        assert!(local_fact
            .bindings
            .iter()
            .all(|def| names[*def].kind == DefKind::Local));
    }

    #[test]
    fn resolves_simple_body_types_through_logic_equalities() {
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
            hir.body()[block].stmts.as_slice()
        else {
            panic!("expected local statement followed by tail expression");
        };
        let init = local.init.expect("local should have initializer");
        let local_def = local
            .bindings
            .first()
            .copied()
            .expect("local should introduce one binding");

        assert!(infer.type_equal_facts().iter().any(|fact| {
            fact.left == TypeSubject::Def(local_def) && fact.right == TypeSubject::Expr(init)
        }));
        assert_usize(&infer, infer.type_for_def(local_def));
        assert_usize(&infer, infer.type_for_hir_expr(init));
        let tail_ty = infer.type_for_hir_expr(*tail);
        assert_usize(&infer, tail_ty);
        assert!(infer
            .resolved_type_facts()
            .iter()
            .any(|fact| { fact.subject == TypeSubject::Expr(*tail) && Some(fact.ty) == tail_ty }));
        assert!(matches!(names[local_def].kind, DefKind::Local));
    }

    fn assert_usize(infer: &InferDb<'_>, ty: Option<TypeId>) {
        let ty = ty.expect("expected a resolved type");
        assert_eq!(infer[ty], Type::Primitive(PrimitiveType::Usize));
    }
}
