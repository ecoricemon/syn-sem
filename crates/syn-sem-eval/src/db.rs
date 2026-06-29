use crate::{ConstInt, ConstValue};
use syn_sem_common::{CommonCx, Map};
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
    ) -> Self {
        let mut db = Self::default();
        db.collect_const_item_inits(hir);
        db.collect_const_item_values(hir, names, infer);
        db.collect_expr_values(hir, names, infer);
        db
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
    ) -> Option<ConstValue> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstValue::from_hir_lit(lit),
            hir::ConstArg::Expr(expr) => self.value_for_hir_expr(*expr),
            hir::ConstArg::Path { path, scope } => {
                let def = self.resolve_const_item(names, path, *scope)?;
                self.value_for_const_def(def)
            }
        }
    }

    /// Returns the evaluated array length when the value is known as an unsigned integer.
    pub fn array_len_value(&self, expr: hir::ExprId) -> Option<usize> {
        let ConstValue::Int(value) = self.value_for_hir_expr(expr)? else {
            return None;
        };
        match value.primitive {
            PrimitiveType::AbstractInt | PrimitiveType::Usize => usize::try_from(value.value).ok(),
            _ => None,
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
    ) {
        let defs = self.const_item_inits.keys().copied().collect::<Vec<_>>();
        for def in defs {
            self.evaluate_const_item(hir, names, infer, def, &mut Vec::new());
        }
    }

    fn collect_expr_values<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
    ) {
        for expr in hir.exprs() {
            self.evaluate_expr(hir, names, infer, expr.id, &mut Vec::new());
        }
    }

    fn evaluate_expr<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        expr: hir::ExprId,
        stack: &mut Vec<DefId>,
    ) -> Option<ConstValue> {
        if let Some(value) = self.value_for_hir_expr(expr) {
            return Some(value);
        }

        let value = match &hir[expr].kind {
            hir::ExprKind::Binary { op, left, right } => {
                let left = self.evaluate_expr(hir, names, infer, *left, stack)?;
                let right = self.evaluate_expr(hir, names, infer, *right, stack)?;
                Self::evaluate_binary(*op, left, right)?
            }
            hir::ExprKind::Block { block } => {
                self.evaluate_block(hir, names, infer, *block, stack)?
            }
            hir::ExprKind::Cast { expr, ty } => {
                let value = self.evaluate_expr(hir, names, infer, *expr, stack)?;
                let primitive = self.primitive_for_hir_type(hir, *ty)?;
                Self::cast_value(value, primitive)?
            }
            hir::ExprKind::Lit(lit) => ConstValue::from_hir_lit(lit)?,
            hir::ExprKind::Paren { expr } => self.evaluate_expr(hir, names, infer, *expr, stack)?,
            hir::ExprKind::Path(path) => {
                self.evaluate_path(hir, names, infer, path, hir[expr].scope, stack)?
            }
            hir::ExprKind::Unary { op, expr } => {
                let value = self.evaluate_expr(hir, names, infer, *expr, stack)?;
                Self::evaluate_unary(*op, value)?
            }
            hir::ExprKind::Array { .. }
            | hir::ExprKind::Assign { .. }
            | hir::ExprKind::Call { .. }
            | hir::ExprKind::Closure { .. }
            | hir::ExprKind::Const { .. }
            | hir::ExprKind::Field { .. }
            | hir::ExprKind::Index { .. }
            | hir::ExprKind::MethodCall { .. }
            | hir::ExprKind::Reference { .. }
            | hir::ExprKind::Repeat { .. }
            | hir::ExprKind::Return { .. }
            | hir::ExprKind::Struct { .. }
            | hir::ExprKind::Tuple { .. } => return None,
        };
        let value = self.refine_with_infer_expr_type(infer, expr, value)?;

        let old = self.expr_values.insert(expr, value);
        assert!(old.is_none(), "HIR expression ids must be unique");
        Some(value)
    }

    fn evaluate_block<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        block: hir::BlockId,
        stack: &mut Vec<DefId>,
    ) -> Option<ConstValue> {
        let block = &hir.lowered_blocks()[block];
        let tail = block.tail_expr?;
        let [hir::lower::Stmt::Expr(expr)] = block.stmts.as_slice() else {
            return None;
        };
        if *expr != tail {
            return None;
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
    ) -> Option<ConstValue> {
        let def = self.resolve_const_item(names, path, scope)?;
        self.evaluate_const_item(hir, names, infer, def, stack)
    }

    fn evaluate_const_item<'cx>(
        &mut self,
        hir: &hir::Hir<'cx>,
        names: &NameDb<'cx>,
        infer: &InferDb<'cx>,
        def: DefId,
        stack: &mut Vec<DefId>,
    ) -> Option<ConstValue> {
        if stack.contains(&def) {
            return None;
        }

        let init = *self.const_item_inits.get(&def)?;
        if let Some(value) = self.value_for_hir_expr(init) {
            return Some(value);
        }

        stack.push(def);
        let value = self.evaluate_expr(hir, names, infer, init, stack);
        let popped = stack.pop();
        assert_eq!(
            popped,
            Some(def),
            "const evaluation stack must stay balanced"
        );
        let value = value.and_then(|value| self.refine_with_const_item_type(hir, def, value));
        if let Some(value) = value {
            self.const_item_values.insert(def, value);
        }
        value
    }

    fn resolve_const_item<'cx>(
        &self,
        names: &NameDb<'cx>,
        path: &hir::Path<'cx>,
        scope: Option<syn_sem_name::ScopeId>,
    ) -> Option<DefId> {
        if path.qself.is_some() || path.segments.iter().any(|segment| !segment.args.is_empty()) {
            return None;
        }
        let scope = scope?;
        let ResolveResult::Found(def) =
            names.resolve_value_path(scope, path.segments.iter().map(|segment| segment.name))
        else {
            return None;
        };
        (names[def].kind == DefKind::Const).then_some(def)
    }

    fn primitive_for_hir_type(&self, hir: &hir::Hir<'_>, ty: hir::TypeId) -> Option<PrimitiveType> {
        let hir::TypeKind::Path(path) = &hir[ty].kind else {
            return None;
        };
        if path.qself.is_some() {
            return None;
        }
        PrimitiveType::from_hir_path(&path.segments)
    }

    fn refine_with_infer_expr_type(
        &self,
        infer: &InferDb<'_>,
        expr: hir::ExprId,
        value: ConstValue,
    ) -> Option<ConstValue> {
        let Some(ty) = infer.type_for_hir_expr(expr) else {
            return Some(value);
        };
        let Type::Primitive(primitive) = infer[ty] else {
            return Some(value);
        };
        if matches!(
            primitive,
            PrimitiveType::AbstractInt | PrimitiveType::AbstractFloat
        ) {
            return Some(value);
        }
        Self::coerce_value(value, primitive).or(Some(value))
    }

    fn refine_with_const_item_type(
        &self,
        hir: &hir::Hir<'_>,
        def: DefId,
        value: ConstValue,
    ) -> Option<ConstValue> {
        let Some(ty) = self.const_item_types.get(&def).copied() else {
            return Some(value);
        };
        let Some(primitive) = self.primitive_for_hir_type(hir, ty) else {
            return Some(value);
        };
        Self::coerce_value(value, primitive)
    }

    fn cast_value(value: ConstValue, primitive: PrimitiveType) -> Option<ConstValue> {
        Self::coerce_value(value, primitive)
    }

    fn coerce_value(value: ConstValue, primitive: PrimitiveType) -> Option<ConstValue> {
        match (value, primitive) {
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
        }
    }

    fn evaluate_binary(
        op: hir::BinaryOp,
        left: ConstValue,
        right: ConstValue,
    ) -> Option<ConstValue> {
        let (ConstValue::Int(left), ConstValue::Int(right)) = (left, right) else {
            return None;
        };
        let value = match op {
            hir::BinaryOp::Add => left.value.checked_add(right.value)?,
            hir::BinaryOp::Sub => left.value.checked_sub(right.value)?,
            hir::BinaryOp::Mul => left.value.checked_mul(right.value)?,
            hir::BinaryOp::Div => left.value.checked_div(right.value)?,
            hir::BinaryOp::Rem => left.value.checked_rem(right.value)?,
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
            | hir::BinaryOp::Gt => return None,
        };
        Some(ConstValue::Int(ConstInt {
            value,
            primitive: integer_binary_primitive(left.primitive, right.primitive)?,
        }))
    }

    fn evaluate_unary(op: hir::UnaryOp, value: ConstValue) -> Option<ConstValue> {
        match (op, value) {
            (hir::UnaryOp::Not, ConstValue::Bool(value)) => Some(ConstValue::Bool(!value)),
            (hir::UnaryOp::Deref | hir::UnaryOp::Neg, _) | (hir::UnaryOp::Not, _) => None,
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
