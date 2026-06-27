use syn_sem_hir as hir;
use syn_sem_infer::PrimitiveType;

/// Compile-time value known to the evaluator.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConstValue {
    /// Integer value with its current primitive type state.
    Int(ConstInt),
    /// Floating-point value with its current primitive type state.
    Float(ConstFloat),
    /// Boolean literal value.
    Bool(bool),
}

impl ConstValue {
    pub(crate) fn from_hir_lit(lit: &hir::Lit<'_>) -> Option<Self> {
        match lit {
            hir::Lit::Int(value) => value.as_ref().parse().ok().map(|value| {
                Self::Int(ConstInt {
                    value,
                    primitive: PrimitiveType::AbstractInt,
                })
            }),
            hir::Lit::Float(value) => value.as_ref().parse().ok().map(|value| {
                Self::Float(ConstFloat {
                    value,
                    primitive: PrimitiveType::AbstractFloat,
                })
            }),
            hir::Lit::Bool(value) => Some(Self::Bool(*value)),
        }
    }
}

/// Integer constant value plus its current primitive type state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConstInt {
    /// Integer value before signed-width interpretation.
    pub value: u128,
    /// Current integer primitive, such as `abstract_int`, `i32`, or `usize`.
    pub primitive: PrimitiveType,
}

/// Floating-point constant value plus its current primitive type state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConstFloat {
    /// Floating-point value.
    pub value: f64,
    /// Current floating-point primitive, such as `abstract_float` or `f64`.
    pub primitive: PrimitiveType,
}
