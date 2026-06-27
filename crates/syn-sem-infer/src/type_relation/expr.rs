//! HIR expression result type fact derivation.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{InferTypes, Type};
use syn_sem_hir as hir;

/// Derives HIR expression result type equalities from resolved operand types.
///
/// This phase extends the subject equality graph with facts that are not visible from source
/// bindings alone. It reads expression shapes from HIR, asks [`TypeRelationDb`] for already resolved
/// operand types, interns any newly constructed result types, and records the result as another
/// subject equality fact.
pub(crate) struct ExprTypeDeriver<'a, 'cx> {
    hir: &'a hir::Hir<'cx>,
    type_relations: &'a mut TypeRelationDb,
    types: &'a mut InferTypes<'cx>,
}

impl<'a, 'cx> ExprTypeDeriver<'a, 'cx> {
    pub(crate) fn new(
        hir: &'a hir::Hir<'cx>,
        type_relations: &'a mut TypeRelationDb,
        types: &'a mut InferTypes<'cx>,
    ) -> Self {
        Self {
            hir,
            type_relations,
            types,
        }
    }

    /// Runs one expression result type derivation pass.
    ///
    /// For example, given:
    /// ```text
    /// fn f(x: usize) {
    ///     let r = &x;
    /// }
    /// ```
    ///
    /// after type relation resolution has resolved `x -> usize`, this derives:
    /// ```text
    /// operand lookup:      x -> usize
    /// result type:         &x has type &usize
    /// equality fact:       expr(&x) == type(&usize)
    /// next propagation:    r -> &usize
    /// ```
    ///
    /// The return value reports whether new equality facts were added, so callers can repeat
    /// subject propagation and expression derivation until the graph reaches a fixed point.
    pub(crate) fn derive(&mut self) -> bool {
        let mut changed = false;
        for expr in self.hir.exprs() {
            changed |= self.derive_expr(expr);
        }
        changed
    }

    fn derive_expr(&mut self, expr: &hir::Expr<'cx>) -> bool {
        match &expr.kind {
            hir::ExprKind::Reference {
                expr: inner,
                is_mut,
            } => self.derive_reference(expr.id, *inner, *is_mut),
            hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Binary { .. }
            | hir::ExprKind::Block { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Cast { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::Lit(_)
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Paren { .. }
            | hir::ExprKind::Path(_)
            | hir::ExprKind::Repeat { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. }
            | hir::ExprKind::Tuple { .. }
            | hir::ExprKind::Unary { .. } => false,
        }
    }

    fn derive_reference(&mut self, expr: hir::ExprId, inner: hir::ExprId, is_mut: bool) -> bool {
        let Some(elem) = self.type_relations.type_for_hir_expr(inner) else {
            return false;
        };
        let reference = self.types.intern_type(Type::Reference { elem, is_mut });
        self.type_relations.insert_equality(TypeEqualityFact {
            left: TypeSubject::Expr(expr),
            right: TypeSubject::Type(reference),
        })
    }
}
