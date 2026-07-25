use crate::{plan::resolve_const_item, ConstInt, ConstValue, EvalNode, EvalPlan};
use syn_sem_common::{GraphNodeId, Map, MaybeResult, Result};
use syn_sem_hir as hir;
use syn_sem_infer::{InferDb, PrimitiveType, Type};
use syn_sem_name::{DefId, NameDb};

/// Evaluated constant facts collected for upper semantic phases.
#[derive(Debug, Default)]
pub struct EvalDb {
    expr_values: Map<hir::ExprId, ConstValue>,
    const_item_values: Map<DefId, ConstValue>,
}

impl EvalDb {
    /// Builds constant-evaluation facts from a reusable plan and current inference facts.
    ///
    /// The plan selects expressions that Rust requires to be constant, such as const item
    /// initializers, array lengths, const blocks, and const generic arguments. Unsupported runtime
    /// expressions outside its targets do not make semantic analysis fail.
    ///
    /// Successfully evaluated values are stored in the returned database. A target remains absent
    /// when its value is unavailable, including cyclic dependencies, unresolved inputs, arithmetic
    /// failure, and insufficient inference facts. Reaching an unsupported expression or operation
    /// from a target returns an error. Each dependency-ordered plan node is visited once; nodes
    /// blocked by a dependency cycle remain absent.
    pub fn analyze<'cx>(
        plan: &EvalPlan,
        hir: &hir::Hir<'cx>,
        infer: &InferDb<'cx>,
    ) -> Result<Self> {
        let mut db = Self::default();
        for node in plan.order() {
            match plan.graph()[*node] {
                EvalNode::Expr(expr) => {
                    if let Some(value) = db.evaluate_expr(plan, hir, infer, *node, expr)? {
                        let old = db.expr_values.insert(expr, value);
                        assert!(old.is_none(), "planned HIR expression nodes must be unique");
                    }
                }
                EvalNode::Const(def) => {
                    if let Some(value) = db.evaluate_const_item(plan, hir, *node, def)? {
                        let old = db.const_item_values.insert(def, value);
                        assert!(
                            old.is_none(),
                            "planned const definition nodes must be unique"
                        );
                    }
                }
            }
        }
        Ok(db)
    }

    /// Returns the value evaluated for a HIR expression.
    pub fn value_for_hir_expr(&self, expr: hir::ExprId) -> Option<ConstValue> {
        self.expr_values.get(&expr).copied()
    }

    /// Returns the value evaluated for a const item definition.
    pub fn value_for_const_def(&self, def: DefId) -> Option<ConstValue> {
        self.const_item_values.get(&def).copied()
    }

    /// Iterates over evaluated HIR expression values in unspecified order.
    pub fn hir_expr_values(&self) -> impl ExactSizeIterator<Item = (hir::ExprId, ConstValue)> + '_ {
        self.expr_values.iter().map(|(expr, value)| (*expr, *value))
    }

    /// Iterates over evaluated const definition values in unspecified order.
    pub fn const_def_values(&self) -> impl ExactSizeIterator<Item = (DefId, ConstValue)> + '_ {
        self.const_item_values
            .iter()
            .map(|(def, value)| (*def, *value))
    }

    /// Returns the value represented by a HIR const argument.
    pub fn value_for_const_arg<'cx>(
        &self,
        names: &NameDb<'cx>,
        arg: &hir::ConstArg<'cx>,
    ) -> MaybeResult<ConstValue> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstValue::from_hir_lit(lit)
                .map_err(|e| format!("EvalDb::value_for_const_arg: {e}").into()),
            hir::ConstArg::Expr(expr) => Ok(self.value_for_hir_expr(*expr)),
            hir::ConstArg::Path { path, scope } => {
                let def = resolve_const_item(names, path, *scope)?;
                Ok(def.and_then(|def| self.value_for_const_def(def)))
            }
        }
    }

    /// Returns the evaluated array length when the value is known as an unsigned integer.
    pub fn array_len_value(&self, expr: hir::ExprId) -> MaybeResult<usize> {
        let Some(value) = self.value_for_hir_expr(expr) else {
            return Ok(None);
        };
        let ConstValue::Int(value) = value else {
            return Ok(None);
        };
        match value.primitive {
            PrimitiveType::AbstractInt | PrimitiveType::Usize => {
                Ok(usize::try_from(value.value).ok())
            }
            _ => Ok(None),
        }
    }

    fn evaluate_expr<'cx>(
        &self,
        plan: &EvalPlan,
        hir: &hir::Hir<'cx>,
        infer: &InferDb<'cx>,
        node: GraphNodeId,
        expr: hir::ExprId,
    ) -> MaybeResult<ConstValue> {
        let value = match &hir[expr].kind {
            hir::ExprKind::Binary { op, left, right } => {
                let Some(left) = self.expr_dependency(plan, node, *left) else {
                    return Ok(None);
                };
                let Some(right) = self.expr_dependency(plan, node, *right) else {
                    return Ok(None);
                };
                let Some(value) = Self::evaluate_binary(*op, left, right)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Block { block } | hir::ExprKind::Const { block } => {
                let Some(value) = self.evaluate_block(plan, hir, node, *block)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Cast { expr, ty } => {
                let Some(value) = self.expr_dependency(plan, node, *expr) else {
                    return Ok(None);
                };
                let Some(primitive) = self.primitive_for_hir_type(hir, *ty)? else {
                    return Ok(None);
                };
                let Some(value) = Self::cast_value(value, primitive)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Lit(lit) => {
                let Some(value) = ConstValue::from_hir_lit(lit)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Paren { expr } => {
                let Some(value) = self.expr_dependency(plan, node, *expr) else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Path(_) => {
                let Some(value) = self.const_dependency(plan, node) else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Unary { op, expr } => {
                let Some(value) = self.expr_dependency(plan, node, *expr) else {
                    return Ok(None);
                };
                let Some(value) = Self::evaluate_unary(*op, value)? else {
                    return Ok(None);
                };
                value
            }
            kind => {
                return Err(format!(
                    "EvalDb::evaluate_expr: unsupported expression {kind:?} for {expr:?}"
                )
                .into());
            }
        };
        let Some(value) = self.refine_with_infer_expr_type(infer, expr, value)? else {
            return Ok(None);
        };
        Ok(Some(value))
    }

    fn evaluate_block<'cx>(
        &self,
        plan: &EvalPlan,
        hir: &hir::Hir<'cx>,
        node: GraphNodeId,
        block: hir::BlockId,
    ) -> MaybeResult<ConstValue> {
        let block = &hir.lowered_blocks()[block];
        let Some(tail) = block.tail_expr else {
            return Err(format!(
                "EvalDb::evaluate_block: unsupported block without a tail expression for {block:?}"
            )
            .into());
        };
        let [hir::lower::Stmt::Expr(expr)] = block.stmts.as_slice() else {
            return Err(format!(
                "EvalDb::evaluate_block: unsupported statement shape for {block:?}"
            )
            .into());
        };
        if *expr != tail {
            return Ok(None);
        }
        Ok(self.expr_dependency(plan, node, tail))
    }

    fn expr_dependency(
        &self,
        plan: &EvalPlan,
        dependent: GraphNodeId,
        expr: hir::ExprId,
    ) -> Option<ConstValue> {
        let dependency = plan
            .graph()
            .node_id(&EvalNode::Expr(expr))
            .expect("planned expression dependencies must have graph nodes");
        assert!(
            plan.graph().incoming(dependent).contains(&dependency),
            "planned expression dependency must have an edge to its dependent"
        );
        self.value_for_hir_expr(expr)
    }

    fn const_dependency(&self, plan: &EvalPlan, dependent: GraphNodeId) -> Option<ConstValue> {
        let mut dependencies = plan.graph().incoming(dependent).iter().filter_map(|node| {
            let EvalNode::Const(def) = plan.graph()[*node] else {
                return None;
            };
            Some(def)
        });
        let def = dependencies.next()?;
        assert!(
            dependencies.next().is_none(),
            "a planned const path must resolve to at most one definition"
        );
        self.value_for_const_def(def)
    }

    fn evaluate_const_item<'cx>(
        &self,
        plan: &EvalPlan,
        hir: &hir::Hir<'cx>,
        node: GraphNodeId,
        def: DefId,
    ) -> MaybeResult<ConstValue> {
        let Some(init) = plan.const_item_init(def) else {
            return Ok(None);
        };
        let Some(value) = self.expr_dependency(plan, node, init) else {
            return Ok(None);
        };
        self.refine_with_const_item_type(plan, hir, def, value)
    }

    fn primitive_for_hir_type(
        &self,
        hir: &hir::Hir<'_>,
        ty: hir::TypeId,
    ) -> MaybeResult<PrimitiveType> {
        let hir::TypeKind::Path(path) = &hir[ty].kind else {
            return Ok(None);
        };
        if path.qself.is_some() {
            return Err(format!(
                "EvalDb::primitive_for_hir_type: unsupported qualified type {ty:?}"
            )
            .into());
        }
        Ok(PrimitiveType::from_hir_path(&path.segments))
    }

    fn refine_with_infer_expr_type(
        &self,
        infer: &InferDb<'_>,
        expr: hir::ExprId,
        value: ConstValue,
    ) -> MaybeResult<ConstValue> {
        let Some(ty) = infer.type_for_hir_expr(expr) else {
            return Ok(Some(value));
        };
        let Type::Primitive(primitive) = infer[ty] else {
            return Ok(Some(value));
        };
        if matches!(
            primitive,
            PrimitiveType::AbstractInt | PrimitiveType::AbstractFloat
        ) {
            return Ok(Some(value));
        }
        Ok(Self::coerce_value(value, primitive)?.or(Some(value)))
    }

    fn refine_with_const_item_type(
        &self,
        plan: &EvalPlan,
        hir: &hir::Hir<'_>,
        def: DefId,
        value: ConstValue,
    ) -> MaybeResult<ConstValue> {
        let Some(ty) = plan.const_item_type(def) else {
            return Ok(Some(value));
        };
        let Some(primitive) = self.primitive_for_hir_type(hir, ty)? else {
            return Ok(Some(value));
        };
        Self::coerce_value(value, primitive)
    }

    fn cast_value(value: ConstValue, primitive: PrimitiveType) -> MaybeResult<ConstValue> {
        Self::coerce_value(value, primitive)
    }

    fn coerce_value(value: ConstValue, primitive: PrimitiveType) -> MaybeResult<ConstValue> {
        let value = match (value, primitive) {
            (ConstValue::Int(value), primitive) if is_integer_primitive(primitive) => {
                fits_integer_primitive(value.value, primitive).then_some(ConstValue::Int(
                    ConstInt {
                        value: value.value,
                        primitive,
                    },
                ))
            }
            (ConstValue::Float(value), PrimitiveType::F32 | PrimitiveType::F64) => {
                Some(ConstValue::Float(crate::ConstFloat {
                    value: value.value,
                    primitive,
                }))
            }
            (ConstValue::Bool(value), PrimitiveType::Bool) => Some(ConstValue::Bool(value)),
            _ => None,
        };
        Ok(value)
    }

    fn evaluate_binary(
        op: hir::BinaryOp,
        left: ConstValue,
        right: ConstValue,
    ) -> MaybeResult<ConstValue> {
        let (ConstValue::Int(left), ConstValue::Int(right)) = (left, right) else {
            return Ok(None);
        };
        let value = match op {
            hir::BinaryOp::Add => left.value.checked_add(right.value),
            hir::BinaryOp::Sub => left.value.checked_sub(right.value),
            hir::BinaryOp::Mul => left.value.checked_mul(right.value),
            hir::BinaryOp::Div => left.value.checked_div(right.value),
            hir::BinaryOp::Rem => left.value.checked_rem(right.value),
            hir::BinaryOp::And
            | hir::BinaryOp::Or
            | hir::BinaryOp::BitXor
            | hir::BinaryOp::BitAnd
            | hir::BinaryOp::BitOr
            | hir::BinaryOp::Shl
            | hir::BinaryOp::Shr
            | hir::BinaryOp::Eq
            | hir::BinaryOp::Lt
            | hir::BinaryOp::Le
            | hir::BinaryOp::Ne
            | hir::BinaryOp::Ge
            | hir::BinaryOp::Gt => {
                return Err(
                    format!("EvalDb::evaluate_binary: unsupported binary op {op:?}").into(),
                );
            }
        };
        let Some(value) = value else {
            return Ok(None);
        };
        let Some(primitive) = integer_binary_primitive(left.primitive, right.primitive) else {
            return Ok(None);
        };
        Ok(Some(ConstValue::Int(ConstInt { value, primitive })))
    }

    fn evaluate_unary(op: hir::UnaryOp, value: ConstValue) -> MaybeResult<ConstValue> {
        match (op, value) {
            (hir::UnaryOp::Not, ConstValue::Bool(value)) => Ok(Some(ConstValue::Bool(!value))),
            (hir::UnaryOp::Deref | hir::UnaryOp::Neg, _) | (hir::UnaryOp::Not, _) => {
                Err(format!("EvalDb::evaluate_unary: unsupported unary op {op:?}").into())
            }
        }
    }
}

fn integer_binary_primitive(left: PrimitiveType, right: PrimitiveType) -> Option<PrimitiveType> {
    match (left, right) {
        (left, right) if left == right && is_integer_primitive(left) => Some(left),
        (PrimitiveType::AbstractInt, right) if is_integer_primitive(right) => Some(right),
        (left, PrimitiveType::AbstractInt) if is_integer_primitive(left) => Some(left),
        (PrimitiveType::AbstractInt, PrimitiveType::AbstractInt) => {
            Some(PrimitiveType::AbstractInt)
        }
        _ => None,
    }
}

fn is_integer_primitive(primitive: PrimitiveType) -> bool {
    matches!(
        primitive,
        PrimitiveType::AbstractInt
            | PrimitiveType::I8
            | PrimitiveType::I16
            | PrimitiveType::I32
            | PrimitiveType::I64
            | PrimitiveType::I128
            | PrimitiveType::Isize
            | PrimitiveType::U8
            | PrimitiveType::U16
            | PrimitiveType::U32
            | PrimitiveType::U64
            | PrimitiveType::U128
            | PrimitiveType::Usize
    )
}

fn fits_integer_primitive(value: u128, primitive: PrimitiveType) -> bool {
    let max = match primitive {
        PrimitiveType::AbstractInt | PrimitiveType::U128 => u128::MAX,
        PrimitiveType::U8 => u8::MAX as u128,
        PrimitiveType::U16 => u16::MAX as u128,
        PrimitiveType::U32 => u32::MAX as u128,
        PrimitiveType::U64 => u64::MAX as u128,
        PrimitiveType::Usize => usize::MAX as u128,
        PrimitiveType::I8 => i8::MAX as u128,
        PrimitiveType::I16 => i16::MAX as u128,
        PrimitiveType::I32 => i32::MAX as u128,
        PrimitiveType::I64 => i64::MAX as u128,
        PrimitiveType::I128 => i128::MAX as u128,
        PrimitiveType::Isize => isize::MAX as u128,
        _ => return false,
    };
    value <= max
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(value: u128, primitive: PrimitiveType) -> ConstValue {
        ConstValue::Int(ConstInt { value, primitive })
    }

    #[test]
    fn returns_unknown_for_invalid_arithmetic_and_integer_coercion() {
        assert_eq!(
            EvalDb::evaluate_binary(
                hir::BinaryOp::Div,
                int(1, PrimitiveType::Usize),
                int(0, PrimitiveType::Usize),
            )
            .unwrap(),
            None
        );
        assert_eq!(
            EvalDb::evaluate_binary(
                hir::BinaryOp::Add,
                int(u128::MAX, PrimitiveType::U128),
                int(1, PrimitiveType::U128),
            )
            .unwrap(),
            None
        );
        assert_eq!(
            EvalDb::coerce_value(int(300, PrimitiveType::AbstractInt), PrimitiveType::U8).unwrap(),
            None
        );
    }

    #[test]
    fn returns_an_error_for_unsupported_constant_operations() {
        let err = EvalDb::evaluate_binary(
            hir::BinaryOp::Eq,
            int(1, PrimitiveType::AbstractInt),
            int(1, PrimitiveType::AbstractInt),
        )
        .expect_err("unsupported constant operations should return an error");
        assert!(err.to_string().contains("unsupported binary op Eq"));
    }
}
