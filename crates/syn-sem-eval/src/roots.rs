//! Discovery of HIR expressions that require compile-time evaluation.
//!
//! This module selects evaluation roots such as array lengths, const blocks, and const generic
//! arguments. It does not evaluate expressions; [`crate::EvalDb`] recursively evaluates the
//! selected roots and records their values.

use syn_sem_common::Set;
use syn_sem_hir as hir;

pub(crate) fn required_exprs(hir: &hir::Hir<'_>) -> Vec<hir::ExprId> {
    let mut roots = EvalRoots::default();

    for ty in hir.types() {
        match &ty.kind {
            hir::TypeKind::Array { len, .. } => {
                let hir::ArrayLen::Expr(expr) = len;
                roots.insert(*expr);
            }
            hir::TypeKind::Path(path) => roots.collect_path(path),
            hir::TypeKind::Infer
            | hir::TypeKind::Reference { .. }
            | hir::TypeKind::Slice { .. }
            | hir::TypeKind::Tuple { .. } => {}
        }
    }

    for item in hir.items() {
        let generics = match &item.kind {
            hir::ItemKind::Enum { generics, .. }
            | hir::ItemKind::Fn { generics, .. }
            | hir::ItemKind::Impl { generics, .. }
            | hir::ItemKind::Struct { generics, .. }
            | hir::ItemKind::Trait { generics, .. }
            | hir::ItemKind::Type { generics, .. } => Some(generics),
            hir::ItemKind::Const { .. } | hir::ItemKind::Mod { .. } | hir::ItemKind::Use { .. } => {
                None
            }
        };
        if let Some(generics) = generics {
            roots.collect_generics(generics);
        }
    }

    for pat in hir.pats() {
        match &pat.kind {
            hir::PatKind::Path(path) | hir::PatKind::Struct { path, .. } => {
                roots.collect_path(path);
            }
            hir::PatKind::Ident { .. }
            | hir::PatKind::Reference { .. }
            | hir::PatKind::Tuple { .. }
            | hir::PatKind::Type { .. }
            | hir::PatKind::Unsupported => {}
        }
    }

    for expr in hir.exprs() {
        match &expr.kind {
            hir::ExprKind::Const { .. } => roots.insert(expr.id),
            hir::ExprKind::MethodCall { generic_args, .. } => {
                if let Some(args) = generic_args {
                    roots.collect_generic_args(args);
                }
            }
            hir::ExprKind::Path(path) | hir::ExprKind::Struct { path, .. } => {
                roots.collect_path(path);
            }
            hir::ExprKind::Repeat { len, .. } => roots.insert(*len),
            hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Binary { .. }
            | hir::ExprKind::Block { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Cast { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::Paren { .. }
            | hir::ExprKind::Reference { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Tuple { .. }
            | hir::ExprKind::Unary { .. } => {}
        }
    }

    roots.exprs
}

#[derive(Default)]
struct EvalRoots {
    exprs: Vec<hir::ExprId>,
    seen: Set<hir::ExprId>,
}

impl EvalRoots {
    fn insert(&mut self, expr: hir::ExprId) {
        if self.seen.insert(expr) {
            self.exprs.push(expr);
        }
    }

    fn collect_generics(&mut self, generics: &hir::Generics<'_>) {
        for predicate in &generics.predicates {
            let hir::WherePredicate::TypeBound { bounds, .. } = predicate else {
                continue;
            };
            self.collect_bounds(bounds);
        }
    }

    fn collect_bounds(&mut self, bounds: &[hir::TypeParamBound<'_>]) {
        for bound in bounds {
            let hir::TypeParamBound::Trait(path) = bound else {
                continue;
            };
            self.collect_path(path);
        }
    }

    fn collect_path(&mut self, path: &hir::Path<'_>) {
        if let Some(qself) = &path.qself {
            self.collect_path_segments(&qself.trait_path);
        }
        self.collect_path_segments(&path.segments);
    }

    fn collect_path_segments(&mut self, segments: &[hir::PathSegment<'_>]) {
        for segment in segments {
            self.collect_generic_args(&segment.args);
        }
    }

    fn collect_generic_args(&mut self, args: &[hir::GenericArg<'_>]) {
        for arg in args {
            match arg {
                hir::GenericArg::Const(arg) | hir::GenericArg::AssocConst { value: arg, .. } => {
                    self.collect_const_arg(arg);
                }
                hir::GenericArg::Constraint { bounds, .. } => self.collect_bounds(bounds),
                hir::GenericArg::Type(_)
                | hir::GenericArg::AssocType { .. }
                | hir::GenericArg::Unsupported => {}
            }
        }
    }

    fn collect_const_arg(&mut self, arg: &hir::ConstArg<'_>) {
        match arg {
            hir::ConstArg::Expr(expr) => self.insert(*expr),
            hir::ConstArg::Path { path, .. } => self.collect_path(path),
            hir::ConstArg::Lit(_) => {}
        }
    }
}
