use crate::ConstValue;
use syn_sem_common::{CommonCx, Map};
use syn_sem_hir as hir;
use syn_sem_infer::{InferDb, PrimitiveType};
use syn_sem_name::NameDb;

/// Evaluated constant facts collected for upper semantic phases.
#[derive(Debug, Default)]
pub struct EvalDb {
    expr_values: Map<hir::ExprId, ConstValue>,
}

impl EvalDb {
    /// Builds constant-evaluation facts from HIR, name facts, and current inference facts.
    ///
    /// The initial skeleton records literal expression values. Later passes can extend this entry
    /// point with path evaluation, typed arithmetic, const blocks, and fixed-point refinement.
    pub fn analyze<'cx>(
        _ccx: &'cx CommonCx,
        hir: &hir::Hir<'cx>,
        _names: &NameDb<'cx>,
        _infer: &InferDb<'cx>,
    ) -> Self {
        let mut db = Self::default();
        db.collect_literal_exprs(hir);
        db
    }

    /// Returns the value evaluated for a HIR expression.
    pub fn value_for_hir_expr(&self, expr: hir::ExprId) -> Option<ConstValue> {
        self.expr_values.get(&expr).copied()
    }

    /// Returns the value represented by a HIR const argument.
    pub fn value_for_const_arg(&self, arg: &hir::ConstArg<'_>) -> Option<ConstValue> {
        match arg {
            hir::ConstArg::Lit(lit) => ConstValue::from_hir_lit(lit),
            hir::ConstArg::Expr(expr) => self.value_for_hir_expr(*expr),
            hir::ConstArg::Path(_) => None,
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

    fn collect_literal_exprs(&mut self, hir: &hir::Hir<'_>) {
        for expr in hir.exprs() {
            if let hir::ExprKind::Lit(lit) = &expr.kind {
                let Some(value) = ConstValue::from_hir_lit(lit) else {
                    continue;
                };
                let old = self.expr_values.insert(expr.id, value);
                assert!(old.is_none(), "HIR expression ids must be unique");
            }
        }
    }
}
