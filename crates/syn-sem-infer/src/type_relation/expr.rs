//! HIR expression result type fact derivation.

use super::{TypeEqualityFact, TypeRelationDb, TypeSubject};
use crate::{ArrayLen, InferConstFacts, InferTypes, PrimitiveType, Type, TypeId};
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
    const_facts: &'a InferConstFacts,
}

impl<'a, 'cx> ExprTypeDeriver<'a, 'cx> {
    pub(crate) fn new(
        hir: &'a hir::Hir<'cx>,
        type_relations: &'a mut TypeRelationDb,
        types: &'a mut InferTypes<'cx>,
        const_facts: &'a InferConstFacts,
    ) -> Self {
        Self {
            hir,
            type_relations,
            types,
            const_facts,
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
            hir::ExprKind::Array { elems } => self.derive_array(expr.id, elems),
            hir::ExprKind::Binary { op, left, right } => {
                self.derive_binary(expr.id, *op, *left, *right)
            }
            hir::ExprKind::Reference {
                expr: inner,
                is_mut,
            } => self.derive_reference(expr.id, *inner, *is_mut),
            hir::ExprKind::Repeat { expr: elem, len } => self.derive_repeat(expr.id, *elem, *len),
            hir::ExprKind::Tuple { elems } => self.derive_tuple(expr.id, elems),
            hir::ExprKind::Unary { op, expr: inner } => self.derive_unary(expr.id, *op, *inner),
            hir::ExprKind::Assign { .. }
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
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. } => false,
        }
    }

    fn derive_array(&mut self, expr: hir::ExprId, elems: &[hir::ExprId]) -> bool {
        if elems.is_empty() {
            return false;
        }
        let Some(elems) = self.resolved_expr_types(elems) else {
            return false;
        };
        let Some(elem) = self.common_elem_type(&elems) else {
            return false;
        };
        let array = self.types.intern_type(Type::Array {
            elem,
            len: ArrayLen::ConstUsize(elems.len()),
        });
        self.insert_expr_type_equality(expr, array)
    }

    fn derive_binary(
        &mut self,
        expr: hir::ExprId,
        op: hir::BinaryOp,
        left: hir::ExprId,
        right: hir::ExprId,
    ) -> bool {
        match op {
            hir::BinaryOp::Add
            | hir::BinaryOp::Sub
            | hir::BinaryOp::Mul
            | hir::BinaryOp::Div
            | hir::BinaryOp::Rem => {
                self.derive_same_type_binary(expr, left, right, |primitive| primitive.is_numeric())
            }
            hir::BinaryOp::BitXor | hir::BinaryOp::BitAnd | hir::BinaryOp::BitOr => self
                .derive_same_type_binary(expr, left, right, |primitive| {
                    primitive == PrimitiveType::Bool || primitive.is_integer()
                }),
            hir::BinaryOp::Shl | hir::BinaryOp::Shr => self.derive_shift_binary(expr, left, right),
            hir::BinaryOp::And | hir::BinaryOp::Or => {
                let bool_ty = self.bool_type();
                self.insert_expr_type_equality(expr, bool_ty)
                    | self.insert_expr_type_equality(left, bool_ty)
                    | self.insert_expr_type_equality(right, bool_ty)
            }
            hir::BinaryOp::Eq
            | hir::BinaryOp::Lt
            | hir::BinaryOp::Le
            | hir::BinaryOp::Ne
            | hir::BinaryOp::Ge
            | hir::BinaryOp::Gt => {
                let bool_ty = self.bool_type();
                self.insert_expr_type_equality(expr, bool_ty)
                    | self.derive_operand_equality(left, right, |primitive| {
                        primitive == PrimitiveType::Bool || primitive.is_numeric()
                    })
            }
        }
    }

    fn derive_same_type_binary(
        &mut self,
        expr: hir::ExprId,
        left: hir::ExprId,
        right: hir::ExprId,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        if !self.has_concrete_operator_type([expr, left, right], accepts) {
            return false;
        }
        self.insert_expr_equality(expr, left) | self.insert_expr_equality(left, right)
    }

    fn derive_shift_binary(
        &mut self,
        expr: hir::ExprId,
        left: hir::ExprId,
        right: hir::ExprId,
    ) -> bool {
        if !self.has_concrete_operator_type([expr, left], PrimitiveType::is_integer) {
            return false;
        }
        let abstract_int = self.abstract_int_type();
        self.insert_expr_equality(expr, left) | self.insert_expr_type_equality(right, abstract_int)
    }

    fn derive_operand_equality(
        &mut self,
        left: hir::ExprId,
        right: hir::ExprId,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        if !self.has_concrete_operator_type([left, right], accepts) {
            return false;
        }
        self.insert_expr_equality(left, right)
    }

    fn derive_reference(&mut self, expr: hir::ExprId, inner: hir::ExprId, is_mut: bool) -> bool {
        let Some(elem) = self.type_relations.type_for_hir_expr(inner) else {
            return false;
        };
        let reference = self.types.intern_type(Type::Reference { elem, is_mut });
        self.insert_expr_type_equality(expr, reference)
    }

    fn derive_repeat(&mut self, expr: hir::ExprId, elem: hir::ExprId, len: hir::ExprId) -> bool {
        let Some(elem) = self.type_relations.type_for_hir_expr(elem) else {
            return false;
        };
        let len = self
            .const_facts
            .expect_integer(len)
            .map(ArrayLen::ConstUsize)
            .unwrap_or(ArrayLen::Expr(len));
        let array = self.types.intern_type(Type::Array { elem, len });
        self.insert_expr_type_equality(expr, array)
    }

    fn derive_tuple(&mut self, expr: hir::ExprId, elems: &[hir::ExprId]) -> bool {
        let Some(elems) = self.resolved_expr_types(elems) else {
            return false;
        };
        let tuple = self.types.intern_type(Type::Tuple { elems });
        self.insert_expr_type_equality(expr, tuple)
    }

    fn derive_unary(&mut self, expr: hir::ExprId, op: hir::UnaryOp, inner: hir::ExprId) -> bool {
        let Some(inner_ty) = self.type_relations.type_for_hir_expr(inner) else {
            return false;
        };
        let Some(primitive) = self.primitive(inner_ty) else {
            return false;
        };

        match op {
            hir::UnaryOp::Not if primitive == PrimitiveType::Bool || primitive.is_integer() => {
                self.insert_expr_type_equality(expr, inner_ty)
            }
            hir::UnaryOp::Neg if primitive.is_numeric() => {
                self.insert_expr_type_equality(expr, inner_ty)
            }
            hir::UnaryOp::Deref | hir::UnaryOp::Not | hir::UnaryOp::Neg => false,
        }
    }

    fn resolved_expr_types(&self, exprs: &[hir::ExprId]) -> Option<Vec<TypeId>> {
        exprs
            .iter()
            .map(|expr| self.type_relations.type_for_hir_expr(*expr))
            .collect()
    }

    fn common_elem_type(&self, elems: &[TypeId]) -> Option<TypeId> {
        let (&first, rest) = elems.split_first()?;
        rest.iter()
            .all(|elem| first == *elem || self.types[first] == self.types[*elem])
            .then_some(first)
    }

    fn primitive(&self, ty: TypeId) -> Option<PrimitiveType> {
        match self.types[ty] {
            Type::Primitive(primitive) => Some(primitive),
            _ => None,
        }
    }

    fn has_concrete_operator_type(
        &self,
        exprs: impl IntoIterator<Item = hir::ExprId>,
        accepts: impl Fn(PrimitiveType) -> bool,
    ) -> bool {
        exprs.into_iter().any(|expr| {
            let Some(primitive) = self
                .type_relations
                .type_for_hir_expr(expr)
                .and_then(|ty| self.primitive(ty))
            else {
                return false;
            };
            accepts(primitive)
                && !matches!(
                    primitive,
                    PrimitiveType::AbstractInt | PrimitiveType::AbstractFloat
                )
        })
    }

    fn abstract_int_type(&mut self) -> TypeId {
        self.types
            .intern_type(Type::Primitive(PrimitiveType::AbstractInt))
    }

    fn bool_type(&mut self) -> TypeId {
        self.types.intern_type(Type::Primitive(PrimitiveType::Bool))
    }

    fn insert_expr_equality(&mut self, left: hir::ExprId, right: hir::ExprId) -> bool {
        self.type_relations.insert_equality(TypeEqualityFact {
            left: TypeSubject::Expr(left),
            right: TypeSubject::Expr(right),
        })
    }

    fn insert_expr_type_equality(&mut self, expr: hir::ExprId, ty: TypeId) -> bool {
        self.type_relations.insert_equality(TypeEqualityFact {
            left: TypeSubject::Expr(expr),
            right: TypeSubject::Type(ty),
        })
    }
}

impl PrimitiveType {
    fn is_integer(self) -> bool {
        matches!(
            self,
            Self::AbstractInt
                | Self::I8
                | Self::I16
                | Self::I32
                | Self::I64
                | Self::I128
                | Self::Isize
                | Self::U8
                | Self::U16
                | Self::U32
                | Self::U64
                | Self::U128
                | Self::Usize
        )
    }

    fn is_numeric(self) -> bool {
        self.is_integer() || matches!(self, Self::AbstractFloat | Self::F32 | Self::F64)
    }
}
