use crate::{ds::vec::BoxedSlice, err, semantic::tree::NodeIndex, Result};
use any_intern::Interned;
use std::{fmt::Debug, iter};

#[derive(Debug, Clone, PartialEq)]
pub enum Value<'gcx> {
    ConstGeneric(ConstGeneric),
    Composed(Vec<Field<'gcx>>),
    Enum(Enum),
    Fn(Fn),
    Ref(Box<Value<'gcx>>),
    Scalar(Scalar),
    Unit,
}

impl<'gcx> Value<'gcx> {
    pub(crate) fn contains_const_generic(&self) -> bool {
        match self {
            Self::ConstGeneric(_) => true,
            Self::Composed(fields) => fields
                .iter()
                .any(|field| field.value.contains_const_generic()),
            Self::Ref(inner) => inner.contains_const_generic(),
            Self::Enum(_) | Self::Fn(_) | Self::Scalar(_) | Self::Unit => false,
        }
    }

    pub(crate) fn iter_const_generic(&self) -> Box<dyn Iterator<Item = &ConstGeneric> + '_> {
        match self {
            Self::ConstGeneric(g) => Box::new(iter::once(g)),
            Self::Composed(fields) => {
                let iter = fields
                    .iter()
                    .flat_map(|field| field.value.iter_const_generic());
                Box::new(iter)
            }
            Self::Ref(inner) => inner.iter_const_generic(),
            Self::Enum(_) | Self::Fn(_) | Self::Scalar(_) | Self::Unit => Box::new(iter::empty()),
        }
    }

    pub(crate) fn try_add(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_add(*r)?)),
            (Self::Ref(l), r) => l.try_add(r),
            (l, Self::Ref(r)) => l.try_add(r),
            _ => err!("cannot apply `+` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_sub(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_sub(*r)?)),
            (Self::Ref(l), r) => l.try_sub(r),
            (l, Self::Ref(r)) => l.try_sub(r),
            _ => err!("cannot apply `-` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_mul(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_mul(*r)?)),
            (Self::Ref(l), r) => l.try_mul(r),
            (l, Self::Ref(r)) => l.try_mul(r),
            _ => err!("cannot apply `*` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_div(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_div(*r)?)),
            (Self::Ref(l), r) => l.try_div(r),
            (l, Self::Ref(r)) => l.try_div(r),
            _ => err!("cannot apply `/` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_rem(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_rem(*r)?)),
            (Self::Ref(l), r) => l.try_rem(r),
            (l, Self::Ref(r)) => l.try_rem(r),
            _ => err!("cannot apply `%` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_xor(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_bit_xor(*r)?)),
            (Self::Ref(l), r) => l.try_bit_xor(r),
            (l, Self::Ref(r)) => l.try_bit_xor(r),
            _ => err!("cannot apply `^` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_and(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_bit_and(*r)?)),
            (Self::Ref(l), r) => l.try_bit_and(r),
            (l, Self::Ref(r)) => l.try_bit_and(r),
            _ => err!("cannot apply `&` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_or(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_bit_or(*r)?)),
            (Self::Ref(l), r) => l.try_bit_or(r),
            (l, Self::Ref(r)) => l.try_bit_or(r),
            _ => err!("cannot apply `|` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_shl(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_shl(*r)?)),
            (Self::Ref(l), r) => l.try_shl(r),
            (l, Self::Ref(r)) => l.try_shl(r),
            _ => err!("cannot apply `<<` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_shr(&self, rhs: &Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Scalar(l), Self::Scalar(r)) => Ok(Self::Scalar(l.try_shr(*r)?)),
            (Self::Ref(l), r) => l.try_shr(r),
            (l, Self::Ref(r)) => l.try_shr(r),
            _ => err!("cannot apply `>>` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_not(&self) -> Result<Self> {
        match self {
            Self::Scalar(v) => Ok(Self::Scalar(v.try_not()?)),
            Self::Ref(v) => v.try_not(),
            _ => err!("cannot apply `!` to {self:?}"),
        }
    }

    pub(crate) fn try_neg(&self) -> Result<Self> {
        match self {
            Self::Scalar(v) => Ok(Self::Scalar(v.try_neg()?)),
            Self::Ref(v) => v.try_neg(),
            _ => err!("cannot apply `-` to {self:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field<'gcx> {
    pub(crate) name: Interned<'gcx, str>,
    pub(crate) value: Value<'gcx>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Scalar {
    Int(i32),
    Float(f32),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    Isize(isize),
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    Usize(usize),
    F32(f32),
    F64(f64),
    Bool(bool),
}

impl Scalar {
    pub(crate) fn try_add(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l + r)),
            (Self::Float(l), Self::Float(r)) => Ok(Self::Float(l + r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l + r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l + r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l + r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l + r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l + r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l + r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l + r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l + r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l + r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l + r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l + r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l + r)),
            (Self::F32(l), Self::F32(r)) => Ok(Self::F32(l + r)),
            (Self::F64(l), Self::F64(r)) => Ok(Self::F64(l + r)),
            _ => err!("cannot apply `+` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_sub(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l - r)),
            (Self::Float(l), Self::Float(r)) => Ok(Self::Float(l - r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l - r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l - r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l - r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l - r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l - r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l - r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l - r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l - r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l - r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l - r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l - r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l - r)),
            (Self::F32(l), Self::F32(r)) => Ok(Self::F32(l - r)),
            (Self::F64(l), Self::F64(r)) => Ok(Self::F64(l - r)),
            _ => err!("cannot apply `-` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_mul(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l * r)),
            (Self::Float(l), Self::Float(r)) => Ok(Self::Float(l * r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l * r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l * r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l * r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l * r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l * r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l * r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l * r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l * r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l * r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l * r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l * r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l * r)),
            (Self::F32(l), Self::F32(r)) => Ok(Self::F32(l * r)),
            (Self::F64(l), Self::F64(r)) => Ok(Self::F64(l * r)),
            _ => err!("cannot apply `*` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_div(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l / r)),
            (Self::Float(l), Self::Float(r)) => Ok(Self::Float(l / r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l / r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l / r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l / r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l / r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l / r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l / r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l / r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l / r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l / r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l / r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l / r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l / r)),
            (Self::F32(l), Self::F32(r)) => Ok(Self::F32(l / r)),
            (Self::F64(l), Self::F64(r)) => Ok(Self::F64(l / r)),
            _ => err!("cannot apply `/` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_rem(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l % r)),
            (Self::Float(l), Self::Float(r)) => Ok(Self::Float(l % r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l % r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l % r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l % r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l % r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l % r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l % r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l % r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l % r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l % r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l % r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l % r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l % r)),
            (Self::F32(l), Self::F32(r)) => Ok(Self::F32(l % r)),
            (Self::F64(l), Self::F64(r)) => Ok(Self::F64(l % r)),
            _ => err!("cannot apply `%` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_xor(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l ^ r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l ^ r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l ^ r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l ^ r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l ^ r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l ^ r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l ^ r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l ^ r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l ^ r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l ^ r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l ^ r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l ^ r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l ^ r)),
            _ => err!("cannot apply `^` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_and(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l & r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l & r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l & r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l & r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l & r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l & r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l & r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l & r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l & r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l & r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l & r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l & r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l & r)),
            _ => err!("cannot apply `&` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_bit_or(self, rhs: Self) -> Result<Self> {
        match (self, rhs) {
            (Self::Int(l), Self::Int(r)) => Ok(Self::Int(l | r)),
            (Self::I8(l), Self::I8(r)) => Ok(Self::I8(l | r)),
            (Self::I16(l), Self::I16(r)) => Ok(Self::I16(l | r)),
            (Self::I32(l), Self::I32(r)) => Ok(Self::I32(l | r)),
            (Self::I64(l), Self::I64(r)) => Ok(Self::I64(l | r)),
            (Self::I128(l), Self::I128(r)) => Ok(Self::I128(l | r)),
            (Self::Isize(l), Self::Isize(r)) => Ok(Self::Isize(l | r)),
            (Self::U8(l), Self::U8(r)) => Ok(Self::U8(l | r)),
            (Self::U16(l), Self::U16(r)) => Ok(Self::U16(l | r)),
            (Self::U32(l), Self::U32(r)) => Ok(Self::U32(l | r)),
            (Self::U64(l), Self::U64(r)) => Ok(Self::U64(l | r)),
            (Self::U128(l), Self::U128(r)) => Ok(Self::U128(l | r)),
            (Self::Usize(l), Self::Usize(r)) => Ok(Self::Usize(l | r)),
            _ => err!("cannot apply `|` to {self:?} and {rhs:?}"),
        }
    }

    pub(crate) fn try_shl(self, rhs: Self) -> Result<Self> {
        return match self {
            Self::Int(l) => Ok(Self::Int(helper(l, rhs)?)),
            Self::I8(l) => Ok(Self::I8(helper(l, rhs)?)),
            Self::I16(l) => Ok(Self::I16(helper(l, rhs)?)),
            Self::I32(l) => Ok(Self::I32(helper(l, rhs)?)),
            Self::I64(l) => Ok(Self::I64(helper(l, rhs)?)),
            Self::I128(l) => Ok(Self::I128(helper(l, rhs)?)),
            Self::Isize(l) => Ok(Self::Isize(helper(l, rhs)?)),
            Self::U8(l) => Ok(Self::U8(helper(l, rhs)?)),
            Self::U16(l) => Ok(Self::U16(helper(l, rhs)?)),
            Self::U32(l) => Ok(Self::U32(helper(l, rhs)?)),
            Self::U64(l) => Ok(Self::U64(helper(l, rhs)?)),
            Self::U128(l) => Ok(Self::U128(helper(l, rhs)?)),
            Self::Usize(l) => Ok(Self::Usize(helper(l, rhs)?)),
            _ => err!("cannot apply `<<` to {self:?} and {rhs:?}"),
        };

        // === Internal helper functions ===

        use std::ops::Shl;
        fn helper<L>(l: L, rhs: Scalar) -> Result<L>
        where
            L: Shl<i8, Output = L>
                + Shl<i16, Output = L>
                + Shl<i32, Output = L>
                + Shl<i64, Output = L>
                + Shl<i128, Output = L>
                + Shl<isize, Output = L>
                + Shl<u8, Output = L>
                + Shl<u16, Output = L>
                + Shl<u32, Output = L>
                + Shl<u64, Output = L>
                + Shl<u128, Output = L>
                + Shl<usize, Output = L>
                + Debug,
        {
            match rhs {
                Scalar::Int(r) => Ok(l << r),
                Scalar::I8(r) => Ok(l << r),
                Scalar::I16(r) => Ok(l << r),
                Scalar::I32(r) => Ok(l << r),
                Scalar::I64(r) => Ok(l << r),
                Scalar::I128(r) => Ok(l << r),
                Scalar::Isize(r) => Ok(l << r),
                Scalar::U8(r) => Ok(l << r),
                Scalar::U16(r) => Ok(l << r),
                Scalar::U32(r) => Ok(l << r),
                Scalar::U64(r) => Ok(l << r),
                Scalar::U128(r) => Ok(l << r),
                Scalar::Usize(r) => Ok(l << r),
                _ => err!("cannot apply `<<` to {l:?} and {rhs:?}"),
            }
        }
    }

    pub(crate) fn try_shr(self, rhs: Self) -> Result<Self> {
        return match self {
            Self::Int(l) => Ok(Self::Int(helper(l, rhs)?)),
            Self::I8(l) => Ok(Self::I8(helper(l, rhs)?)),
            Self::I16(l) => Ok(Self::I16(helper(l, rhs)?)),
            Self::I32(l) => Ok(Self::I32(helper(l, rhs)?)),
            Self::I64(l) => Ok(Self::I64(helper(l, rhs)?)),
            Self::I128(l) => Ok(Self::I128(helper(l, rhs)?)),
            Self::Isize(l) => Ok(Self::Isize(helper(l, rhs)?)),
            Self::U8(l) => Ok(Self::U8(helper(l, rhs)?)),
            Self::U16(l) => Ok(Self::U16(helper(l, rhs)?)),
            Self::U32(l) => Ok(Self::U32(helper(l, rhs)?)),
            Self::U64(l) => Ok(Self::U64(helper(l, rhs)?)),
            Self::U128(l) => Ok(Self::U128(helper(l, rhs)?)),
            Self::Usize(l) => Ok(Self::Usize(helper(l, rhs)?)),
            _ => err!("cannot apply `>>` to {self:?} and {rhs:?}"),
        };

        // === Internal helper functions ===

        use std::ops::Shr;
        fn helper<L>(l: L, rhs: Scalar) -> Result<L>
        where
            L: Shr<i8, Output = L>
                + Shr<i16, Output = L>
                + Shr<i32, Output = L>
                + Shr<i64, Output = L>
                + Shr<i128, Output = L>
                + Shr<isize, Output = L>
                + Shr<u8, Output = L>
                + Shr<u16, Output = L>
                + Shr<u32, Output = L>
                + Shr<u64, Output = L>
                + Shr<u128, Output = L>
                + Shr<usize, Output = L>
                + Debug,
        {
            match rhs {
                Scalar::Int(r) => Ok(l >> r),
                Scalar::I8(r) => Ok(l >> r),
                Scalar::I16(r) => Ok(l >> r),
                Scalar::I32(r) => Ok(l >> r),
                Scalar::I64(r) => Ok(l >> r),
                Scalar::I128(r) => Ok(l >> r),
                Scalar::Isize(r) => Ok(l >> r),
                Scalar::U8(r) => Ok(l >> r),
                Scalar::U16(r) => Ok(l >> r),
                Scalar::U32(r) => Ok(l >> r),
                Scalar::U64(r) => Ok(l >> r),
                Scalar::U128(r) => Ok(l >> r),
                Scalar::Usize(r) => Ok(l >> r),
                _ => err!("cannot apply `>>` to {l:?} and {rhs:?}"),
            }
        }
    }

    pub(crate) fn try_not(self) -> Result<Self> {
        if let Self::Bool(v) = self {
            Ok(Self::Bool(!v))
        } else {
            err!("cannot apply `!` to {self:?}")
        }
    }

    pub(crate) fn try_neg(self) -> Result<Self> {
        match self {
            Self::Int(v) => Ok(Self::Int(-v)),
            Self::Float(v) => Ok(Self::Float(-v)),
            Self::I8(v) => Ok(Self::I8(-v)),
            Self::I16(v) => Ok(Self::I16(-v)),
            Self::I32(v) => Ok(Self::I32(-v)),
            Self::I64(v) => Ok(Self::I64(-v)),
            Self::I128(v) => Ok(Self::I128(-v)),
            Self::Isize(v) => Ok(Self::Isize(-v)),
            Self::F32(v) => Ok(Self::F32(-v)),
            Self::F64(v) => Ok(Self::F64(-v)),
            _ => err!("cannot apply `-` to {self:?}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstGeneric {
    pub expr: *const syn::Expr,
    pub base: NodeIndex,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Enum {
    pub path: String,
    pub disc: isize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fn {
    pub inputs: FnInputs,
    pub output: *const syn::ReturnType,
    pub body: FnBody,
}

impl Fn {
    pub(crate) fn from_signature_and_block(sig: &syn::Signature, block: &syn::Block) -> Self {
        let inputs = sig
            .inputs
            .iter()
            .map(|input| input as *const syn::FnArg)
            .collect();
        let inputs = FnInputs::Params(inputs);
        let output = &sig.output as *const _;
        let body = FnBody::Block(block as *const _);
        Self {
            inputs,
            output,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnInputs {
    Params(BoxedSlice<*const syn::FnArg>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FnBody {
    Block(*const syn::Block),
}
