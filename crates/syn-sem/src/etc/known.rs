pub(crate) fn scalar_names() -> [&'static str; 15] {
    [
        i8::name(),
        i16::name(),
        i32::name(),
        i64::name(),
        i128::name(),
        isize::name(),
        u8::name(),
        u16::name(),
        u32::name(),
        u64::name(),
        u128::name(),
        usize::name(),
        f32::name(),
        f64::name(),
        bool::name(),
    ]
}

trait Name {
    fn name() -> &'static str;
}

macro_rules! impl_name {
    ($($ty:ty)*) => {$(
        impl Name for $ty {
            fn name() -> &'static str {
                stringify!($ty)
            }
        }
    )*};
}

impl_name!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64 bool);

macro_rules! impl_bin_op {
    ($trait:ident, $fn_name:ident, $self:ty, $rhs:ty) => {stringify!(
        impl $trait<$rhs> for $self {
            type Output = $self;
            fn $fn_name(self, rhs: $rhs) -> $self {}
        }
        impl $trait<&$rhs> for $self {
            type Output = $self;
            fn $fn_name(self, rhs: &$rhs) -> $self {}
        }
        impl $trait<$rhs> for &$self {
            type Output = $self;
            fn $fn_name(self, rhs: $rhs) -> $self {}
        }
        impl $trait<&$rhs> for &$self {
            type Output = $self;
            fn $fn_name(self, rhs: &$rhs) -> $self {}
        }
    )};
}

macro_rules! impl_bin_assign_op {
    ($trait:ident, $fn_name:ident, $self:ty, $rhs:ty) => {stringify!(
        impl $trait<$rhs> for $self {
            fn $fn_name(&mut self, rhs: $rhs) {}
        }
        impl $trait<&$rhs> for $self {
            fn $fn_name(&mut self, rhs: &$rhs) {}
        }
    )};
}

macro_rules! impl_unary_op {
    ($trait:ident, $fn_name:ident, $self:ty) => {stringify!(
        impl $trait for $self {
            type Output = $self;
            fn $fn_name(self) -> $self {}
        }
        impl $trait for &$self {
            type Output = $self;
            fn $fn_name(self) -> $self {}
        }
    )};
}

macro_rules! impl_add {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Add<Rhs = Self> { type Output; fn add(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(Add, add, $self, $self) ),*
    )};
}

macro_rules! impl_sub {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Sub<Rhs = Self> { type Output; fn sub(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(Sub, sub, $self, $self) ),*
    )};
}

macro_rules! impl_mul {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Mul<Rhs = Self> { type Output; fn mul(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(Mul, mul, $self, $self) ),*
    )};
}

macro_rules! impl_div {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Div<Rhs = Self> { type Output; fn div(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(Div, div, $self, $self) ),*
    )};
}

macro_rules! impl_rem {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Rem<Rhs = Self> { type Output; fn rem(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(Rem, rem, $self, $self) ),*
    )};
}

macro_rules! impl_bitxor {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitXor<Rhs = Self> { type Output; fn bitxor(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(BitXor, bitxor, $self, $self) ),*
    )};
}

macro_rules! impl_bitand {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitAnd<Rhs = Self> { type Output; fn bitand(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(BitAnd, bitand, $self, $self) ),*
    )};
}

macro_rules! impl_bitor {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitOr<Rhs = Self> { type Output; fn bitor(self, rhs: Rhs) -> Self::Output; }",
        $( impl_bin_op!(BitOr, bitor, $self, $self) ),*
    )};
}

macro_rules! impl_shl {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Shl<Rhs = Self> { type Output; fn shl(self, rhs: Rhs) -> Self::Output; }",
        $(
            impl_bin_op!(Shl, shl, $self, i8),
            impl_bin_op!(Shl, shl, $self, i16),
            impl_bin_op!(Shl, shl, $self, i32),
            impl_bin_op!(Shl, shl, $self, i64),
            impl_bin_op!(Shl, shl, $self, i128),
            impl_bin_op!(Shl, shl, $self, isize),
            impl_bin_op!(Shl, shl, $self, u8),
            impl_bin_op!(Shl, shl, $self, u16),
            impl_bin_op!(Shl, shl, $self, u32),
            impl_bin_op!(Shl, shl, $self, u64),
            impl_bin_op!(Shl, shl, $self, u128),
            impl_bin_op!(Shl, shl, $self, usize)
        ),*
    )};
}

macro_rules! impl_shr {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Shr<Rhs = Self> { type Output; fn shr(self, rhs: Rhs) -> Self::Output; }",
        $(
            impl_bin_op!(Shr, shr, $self, i8),
            impl_bin_op!(Shr, shr, $self, i16),
            impl_bin_op!(Shr, shr, $self, i32),
            impl_bin_op!(Shr, shr, $self, i64),
            impl_bin_op!(Shr, shr, $self, i128),
            impl_bin_op!(Shr, shr, $self, isize),
            impl_bin_op!(Shr, shr, $self, u8),
            impl_bin_op!(Shr, shr, $self, u16),
            impl_bin_op!(Shr, shr, $self, u32),
            impl_bin_op!(Shr, shr, $self, u64),
            impl_bin_op!(Shr, shr, $self, u128),
            impl_bin_op!(Shr, shr, $self, usize)
        ),*
    )};
}

macro_rules! impl_addassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait AddAssign<Rhs = Self> { fn add_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(AddAssign, add_assign, $self, $self) ),*
    )};
}

macro_rules! impl_subassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait SubAssign<Rhs = Self> { fn sub_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(SubAssign, sub_assign, $self, $self) ),*
    )};
}

macro_rules! impl_mulassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait MulAssign<Rhs = Self> { fn mul_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(MulAssign, mul_assign, $self, $self) ),*
    )};
}

macro_rules! impl_divassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait DivAssign<Rhs = Self> { fn div_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(DivAssign, div_assign, $self, $self) ),*
    )};
}

macro_rules! impl_remassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait RemAssign<Rhs = Self> { fn rem_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(RemAssign, rem_assign, $self, $self) ),*
    )};
}

macro_rules! impl_bitxorassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitXorAssign<Rhs = Self> { fn bitxor_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(BitXorAssign, bitxor_assign, $self, $self) ),*
    )};
}

macro_rules! impl_bitandassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitAndAssign<Rhs = Self> { fn bitand_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(BitAndAssign, bitand_assign, $self, $self) ),*
    )};
}

macro_rules! impl_bitorassign {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait BitOrAssign<Rhs = Self> { fn bitor_assign(&mut self, rhs: Rhs); }",
        $( impl_bin_assign_op!(BitOrAssign, bitor_assign, $self, $self) ),*
    )};
}

macro_rules! impl_not {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Not { type Output; fn not(self) -> Self::Output; }",
        $( impl_unary_op!(Not, not, $self) ),*
    )};
}

macro_rules! impl_neg {
    ($($self:ty)*) => {const_format::concatcp!(
        "pub trait Neg { type Output; fn neg(self) -> Self::Output; }",
        $( impl_unary_op!(Neg, neg, $self) ),*
    )};
}

pub(crate) const LIB_CORE_CODE: &str = const_format::concatcp!(
    "pub mod ops {",
    impl_add!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_sub!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_mul!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_div!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_rem!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_bitxor!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_bitand!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_bitor!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_shl!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize),
    impl_shr!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize),
    impl_addassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_subassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_mulassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_divassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_remassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize f32 f64),
    impl_bitxorassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_bitandassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_bitorassign!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_not!(i8 i16 i32 i64 i128 isize u8 u16 u32 u64 u128 usize bool),
    impl_neg!(i8 i16 i32 i64 i128 isize f32 f64),
    "}",
);

pub(crate) const LIB_STD_CODE: &str = "
pub use core::ops;
";

pub(crate) mod apply {
    pub(crate) const NAME_ADD: &str = "core::ops::Add::add";
    pub(crate) const NAME_SUB: &str = "core::ops::Sub::sub";
    pub(crate) const NAME_MUL: &str = "core::ops::Mul::mul";
    pub(crate) const NAME_DIV: &str = "core::ops::Div::div";
    pub(crate) const NAME_REM: &str = "core::ops::Rem::rem";
    pub(crate) const NAME_BIT_XOR: &str = "core::ops::BitXor::bitxor";
    pub(crate) const NAME_BIT_AND: &str = "core::ops::BitAnd::bitand";
    pub(crate) const NAME_BIT_OR: &str = "core::ops::BitOr::bitor";
    pub(crate) const NAME_SHL: &str = "core::ops::Shl::shl";
    pub(crate) const NAME_SHR: &str = "core::ops::Shr::shr";
    pub(crate) const NAME_ADD_ASSIGN: &str = "core::ops::AddAssign::add_assign";
    pub(crate) const NAME_SUB_ASSIGN: &str = "core::ops::SubAssign::sub_assign";
    pub(crate) const NAME_MUL_ASSIGN: &str = "core::ops::MulAssign::mul_assign";
    pub(crate) const NAME_DIV_ASSIGN: &str = "core::ops::DivAssign::div_assign";
    pub(crate) const NAME_REM_ASSIGN: &str = "core::ops::RemAssign::rem_assign";
    pub(crate) const NAME_BIT_XOR_ASSIGN: &str = "core::ops::BitXorAssign::bitxor_assign";
    pub(crate) const NAME_BIT_AND_ASSIGN: &str = "core::ops::BitAndAssign::bitand_assign";
    pub(crate) const NAME_BIT_OR_ASSIGN: &str = "core::ops::BitOrAssign::bitor_assign";
    pub(crate) const NAME_SHL_ASSIGN: &str = "core::ops::ShlAssign::shl_assign";
    pub(crate) const NAME_SHR_ASSIGN: &str = "core::ops::ShrAssign::shr_assign";
    pub(crate) const NAME_NOT: &str = "core::ops::Not::not";
    pub(crate) const NAME_NEG: &str = "core::ops::Neg::neg";
}
