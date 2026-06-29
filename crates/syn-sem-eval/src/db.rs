use crate::{ConstInt, ConstValue};
use syn_sem_common::{CommonCx, Map, MaybeResult, Result};
use syn_sem_hir as hir;
use syn_sem_infer::{InferDb, PrimitiveType, Type};
use syn_sem_name::{DefId, DefKind, NameDb, ResolveResult};

/// Evaluated constant facts collected for upper semantic phases.
#[derive(Debug, Default)]
pub struct EvalDb {
    expr_values: Map<hir::ExprId, ConstValue>,
    const_item_inits: Map<DefId, hir::ExprId>,
    const_item_types: Map<DefId, hir::TypeId>,
    const_item_values: Map<DefId, ConstValue>,
}

impl EvalDb {
    /// Builds constant-evaluation facts from HIR, name facts, and current inference facts.
    ///
    /// The initial pass records literal values and simple expression values. Later passes can
    /// extend this entry point with path evaluation, typed arithmetic, const blocks, and
    /// fixed-point refinement.
    pub fn analyze<'cx>(
        _ccx: &'cx CommonCx,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
    ) -> Result<Self> {
        let mut db = Self::default();
        db.collect_const_item_inits(hir);
        db.collect_const_item_values(hir, names, infer)?;
        db.collect_expr_values(hir, names, infer)?;
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
                let def = self.resolve_const_item(names, path, *scope)?;
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

    fn collect_const_item_inits(&mut self, hir: &hir::Hir<'_>) {
        for item in hir.items() {
            let hir::ItemKind::Const { ty, init } = item.kind else {
                continue;
            };
            let Some(def) = item.def else {
                continue;
            };
            self.const_item_inits.insert(def, init);
            self.const_item_types.insert(def, ty);
        }
    }

    fn collect_const_item_values<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
    ) -> Result<()> {
        let defs = self.const_item_inits.keys().copied().collect::<Vec<_>>();
        for def in defs {
            self.evaluate_const_item(hir, names, infer, def, &mut Vec::new())?;
        }
        Ok(())
    }

    fn collect_expr_values<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
    ) -> Result<()> {
        for expr in hir.exprs() {
            self.evaluate_expr(hir, names, infer, expr.id, &mut Vec::new())?;
        }
        Ok(())
    }

    fn evaluate_expr<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        expr: hir::ExprId,
        stack: &mut Vec<DefId>,
    ) -> MaybeResult<ConstValue> {
        if let Some(value) = self.value_for_hir_expr(expr) {
            return Ok(Some(value));
        }

        let value = match &hir[expr].kind {
            hir::ExprKind::Binary { op, left, right } => {
                let Some(left) = self.evaluate_expr(hir, names, infer, *left, stack)? else {
                    return Ok(None);
                };
                let Some(right) = self.evaluate_expr(hir, names, infer, *right, stack)? else {
                    return Ok(None);
                };
                let Some(value) = Self::evaluate_binary(*op, left, right)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Block { block } => {
                let Some(value) = self.evaluate_block(hir, names, infer, *block, stack)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Cast { expr, ty } => {
                let Some(value) = self.evaluate_expr(hir, names, infer, *expr, stack)? else {
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
                return self.evaluate_expr(hir, names, infer, *expr, stack);
            }
            hir::ExprKind::Path(path) => {
                let Some(value) =
                    self.evaluate_path(hir, names, infer, path, hir[expr].scope, stack)?
                else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Unary { op, expr } => {
                let Some(value) = self.evaluate_expr(hir, names, infer, *expr, stack)? else {
                    return Ok(None);
                };
                let Some(value) = Self::evaluate_unary(*op, value)? else {
                    return Ok(None);
                };
                value
            }
            hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Reference { .. }
            | hir::ExprKind::Repeat { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. }
            | hir::ExprKind::Tuple { .. } => return Ok(None),
            hir::ExprKind::Closure { .. } => {
                return Err(format!(
                    "EvalDb::evaluate_expr: unsupported closure expression for {expr:?}"
                )
                .into());
            }
        };
        let Some(value) = self.refine_with_infer_expr_type(infer, expr, value)? else {
            return Ok(None);
        };

        let old = self.expr_values.insert(expr, value);
        assert!(old.is_none(), "HIR expression ids must be unique");
        Ok(Some(value))
    }

    fn evaluate_block<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        block: hir::BlockId,
        stack: &mut Vec<DefId>,
    ) -> MaybeResult<ConstValue> {
        let block = &hir.lowered_blocks()[block];
        let Some(tail) = block.tail_expr else {
            return Ok(None);
        };
        let [hir::lower::Stmt::Expr(expr)] = block.stmts.as_slice() else {
            return Ok(None);
        };
        if *expr != tail {
            return Ok(None);
        }
        self.evaluate_expr(hir, names, infer, tail, stack)
    }

    fn evaluate_path<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        path: &hir::Path<'cx>,
        scope: Option<syn_sem_name::ScopeId>,
        stack: &mut Vec<DefId>,
    ) -> MaybeResult<ConstValue> {
        let Some(def) = self.resolve_const_item(names, path, scope)? else {
            return Ok(None);
        };
        self.evaluate_const_item(hir, names, infer, def, stack)
    }

    fn evaluate_const_item<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        def: DefId,
        stack: &mut Vec<DefId>,
    ) -> MaybeResult<ConstValue> {
        if stack.contains(&def) {
            return Ok(None);
        }

        let Some(init) = self.const_item_inits.get(&def).copied() else {
            return Ok(None);
        };
        if let Some(value) = self.value_for_hir_expr(init) {
            return Ok(Some(value));
        }

        stack.push(def);
        let value = self.evaluate_expr(hir, names, infer, init, stack);
        let popped = stack.pop();
        assert_eq!(
            popped,
            Some(def),
            "const evaluation stack must stay balanced"
        );
        let value = match value? {
            Some(value) => self.refine_with_const_item_type(hir, def, value)?,
            None => None,
        };
        if let Some(value) = value {
            self.const_item_values.insert(def, value);
        }
        Ok(value)
    }

    fn resolve_const_item<'cx>(
        &self,
        names: &NameDb<'cx>,
        path: &hir::Path<'cx>,
        scope: Option<syn_sem_name::ScopeId>,
    ) -> MaybeResult<DefId> {
        if path.qself.is_some() || path.segments.iter().any(|segment| !segment.args.is_empty()) {
            return Err("EvalDb::resolve_const_item: unsupported const path shape".into());
        }
        let Some(scope) = scope else {
            return Ok(None);
        };
        let ResolveResult::Found(def) =
            names.resolve_value_path(scope, path.segments.iter().map(|segment| segment.name))
        else {
            return Ok(None);
        };
        Ok((names[def].kind == DefKind::Const).then_some(def))
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
        hir: &hir::Hir<'_>,
        def: DefId,
        value: ConstValue,
    ) -> MaybeResult<ConstValue> {
        let Some(ty) = self.const_item_types.get(&def).copied() else {
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
