use syn_sem_common::MaybeResult;
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
    pub(crate) fn from_hir_lit(lit: &hir::Lit<'_>) -> MaybeResult<Self> {
        match lit {
            hir::Lit::Int(lit) => {
                let value = lit.digits.as_ref().parse().map_err(|e| {
                    format!("ConstValue::from_hir_lit: invalid integer literal: {e}")
                })?;
                let Some(primitive) = integer_suffix_primitive(lit.suffix.as_ref()) else {
                    return Err(format!(
                        "ConstValue::from_hir_lit: unsupported integer suffix `{}`",
                        lit.suffix
                    )
                    .into());
                };
                if !fits_integer_primitive(value, primitive) {
                    return Ok(None);
                }
                Ok(Some(Self::Int(ConstInt { value, primitive })))
            }
            hir::Lit::Float(lit) => {
                let value =
                    lit.digits.as_ref().parse().map_err(|e| {
                        format!("ConstValue::from_hir_lit: invalid float literal: {e}")
                    })?;
                let Some(primitive) = float_suffix_primitive(lit.suffix.as_ref()) else {
                    return Err(format!(
                        "ConstValue::from_hir_lit: unsupported float suffix `{}`",
                        lit.suffix
                    )
                    .into());
                };
                Ok(Some(Self::Float(ConstFloat { value, primitive })))
            }
            hir::Lit::Bool(value) => Ok(Some(Self::Bool(*value))),
        }
    }
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

fn integer_suffix_primitive(suffix: &str) -> Option<PrimitiveType> {
    match suffix {
        "" => Some(PrimitiveType::AbstractInt),
        "i8" => Some(PrimitiveType::I8),
        "i16" => Some(PrimitiveType::I16),
        "i32" => Some(PrimitiveType::I32),
        "i64" => Some(PrimitiveType::I64),
        "i128" => Some(PrimitiveType::I128),
        "isize" => Some(PrimitiveType::Isize),
        "u8" => Some(PrimitiveType::U8),
        "u16" => Some(PrimitiveType::U16),
        "u32" => Some(PrimitiveType::U32),
        "u64" => Some(PrimitiveType::U64),
        "u128" => Some(PrimitiveType::U128),
        "usize" => Some(PrimitiveType::Usize),
        _ => None,
    }
}

fn float_suffix_primitive(suffix: &str) -> Option<PrimitiveType> {
    match suffix {
        "" => Some(PrimitiveType::AbstractFloat),
        "f32" => Some(PrimitiveType::F32),
        "f64" => Some(PrimitiveType::F64),
        _ => None,
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
