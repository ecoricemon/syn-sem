// Direct source model matching the operator coverage of legacy `syn-sem`.
// Keep implementations grouped by trait so additions remain easy to audit.
pub mod core {
    pub mod ops {

        // Arithmetic and bitwise binary operators

        pub trait Add<Rhs = Self> { type Output; fn add(self, rhs: Rhs) -> Self::Output; }
        impl Add<i8> for i8 { type Output = i8; fn add(self, _rhs: i8) -> i8 {} }
        impl Add<&i8> for i8 { type Output = i8; fn add(self, _rhs: &i8) -> i8 {} }
        impl Add<i8> for &i8 { type Output = i8; fn add(self, _rhs: i8) -> i8 {} }
        impl Add<&i8> for &i8 { type Output = i8; fn add(self, _rhs: &i8) -> i8 {} }
        impl Add<i16> for i16 { type Output = i16; fn add(self, _rhs: i16) -> i16 {} }
        impl Add<&i16> for i16 { type Output = i16; fn add(self, _rhs: &i16) -> i16 {} }
        impl Add<i16> for &i16 { type Output = i16; fn add(self, _rhs: i16) -> i16 {} }
        impl Add<&i16> for &i16 { type Output = i16; fn add(self, _rhs: &i16) -> i16 {} }
        impl Add<i32> for i32 { type Output = i32; fn add(self, _rhs: i32) -> i32 {} }
        impl Add<&i32> for i32 { type Output = i32; fn add(self, _rhs: &i32) -> i32 {} }
        impl Add<i32> for &i32 { type Output = i32; fn add(self, _rhs: i32) -> i32 {} }
        impl Add<&i32> for &i32 { type Output = i32; fn add(self, _rhs: &i32) -> i32 {} }
        impl Add<i64> for i64 { type Output = i64; fn add(self, _rhs: i64) -> i64 {} }
        impl Add<&i64> for i64 { type Output = i64; fn add(self, _rhs: &i64) -> i64 {} }
        impl Add<i64> for &i64 { type Output = i64; fn add(self, _rhs: i64) -> i64 {} }
        impl Add<&i64> for &i64 { type Output = i64; fn add(self, _rhs: &i64) -> i64 {} }
        impl Add<i128> for i128 { type Output = i128; fn add(self, _rhs: i128) -> i128 {} }
        impl Add<&i128> for i128 { type Output = i128; fn add(self, _rhs: &i128) -> i128 {} }
        impl Add<i128> for &i128 { type Output = i128; fn add(self, _rhs: i128) -> i128 {} }
        impl Add<&i128> for &i128 { type Output = i128; fn add(self, _rhs: &i128) -> i128 {} }
        impl Add<isize> for isize { type Output = isize; fn add(self, _rhs: isize) -> isize {} }
        impl Add<&isize> for isize { type Output = isize; fn add(self, _rhs: &isize) -> isize {} }
        impl Add<isize> for &isize { type Output = isize; fn add(self, _rhs: isize) -> isize {} }
        impl Add<&isize> for &isize { type Output = isize; fn add(self, _rhs: &isize) -> isize {} }
        impl Add<u8> for u8 { type Output = u8; fn add(self, _rhs: u8) -> u8 {} }
        impl Add<&u8> for u8 { type Output = u8; fn add(self, _rhs: &u8) -> u8 {} }
        impl Add<u8> for &u8 { type Output = u8; fn add(self, _rhs: u8) -> u8 {} }
        impl Add<&u8> for &u8 { type Output = u8; fn add(self, _rhs: &u8) -> u8 {} }
        impl Add<u16> for u16 { type Output = u16; fn add(self, _rhs: u16) -> u16 {} }
        impl Add<&u16> for u16 { type Output = u16; fn add(self, _rhs: &u16) -> u16 {} }
        impl Add<u16> for &u16 { type Output = u16; fn add(self, _rhs: u16) -> u16 {} }
        impl Add<&u16> for &u16 { type Output = u16; fn add(self, _rhs: &u16) -> u16 {} }
        impl Add<u32> for u32 { type Output = u32; fn add(self, _rhs: u32) -> u32 {} }
        impl Add<&u32> for u32 { type Output = u32; fn add(self, _rhs: &u32) -> u32 {} }
        impl Add<u32> for &u32 { type Output = u32; fn add(self, _rhs: u32) -> u32 {} }
        impl Add<&u32> for &u32 { type Output = u32; fn add(self, _rhs: &u32) -> u32 {} }
        impl Add<u64> for u64 { type Output = u64; fn add(self, _rhs: u64) -> u64 {} }
        impl Add<&u64> for u64 { type Output = u64; fn add(self, _rhs: &u64) -> u64 {} }
        impl Add<u64> for &u64 { type Output = u64; fn add(self, _rhs: u64) -> u64 {} }
        impl Add<&u64> for &u64 { type Output = u64; fn add(self, _rhs: &u64) -> u64 {} }
        impl Add<u128> for u128 { type Output = u128; fn add(self, _rhs: u128) -> u128 {} }
        impl Add<&u128> for u128 { type Output = u128; fn add(self, _rhs: &u128) -> u128 {} }
        impl Add<u128> for &u128 { type Output = u128; fn add(self, _rhs: u128) -> u128 {} }
        impl Add<&u128> for &u128 { type Output = u128; fn add(self, _rhs: &u128) -> u128 {} }
        impl Add<usize> for usize { type Output = usize; fn add(self, _rhs: usize) -> usize {} }
        impl Add<&usize> for usize { type Output = usize; fn add(self, _rhs: &usize) -> usize {} }
        impl Add<usize> for &usize { type Output = usize; fn add(self, _rhs: usize) -> usize {} }
        impl Add<&usize> for &usize { type Output = usize; fn add(self, _rhs: &usize) -> usize {} }
        impl Add<f32> for f32 { type Output = f32; fn add(self, _rhs: f32) -> f32 {} }
        impl Add<&f32> for f32 { type Output = f32; fn add(self, _rhs: &f32) -> f32 {} }
        impl Add<f32> for &f32 { type Output = f32; fn add(self, _rhs: f32) -> f32 {} }
        impl Add<&f32> for &f32 { type Output = f32; fn add(self, _rhs: &f32) -> f32 {} }
        impl Add<f64> for f64 { type Output = f64; fn add(self, _rhs: f64) -> f64 {} }
        impl Add<&f64> for f64 { type Output = f64; fn add(self, _rhs: &f64) -> f64 {} }
        impl Add<f64> for &f64 { type Output = f64; fn add(self, _rhs: f64) -> f64 {} }
        impl Add<&f64> for &f64 { type Output = f64; fn add(self, _rhs: &f64) -> f64 {} }

        pub trait Sub<Rhs = Self> { type Output; fn sub(self, rhs: Rhs) -> Self::Output; }
        impl Sub<i8> for i8 { type Output = i8; fn sub(self, _rhs: i8) -> i8 {} }
        impl Sub<&i8> for i8 { type Output = i8; fn sub(self, _rhs: &i8) -> i8 {} }
        impl Sub<i8> for &i8 { type Output = i8; fn sub(self, _rhs: i8) -> i8 {} }
        impl Sub<&i8> for &i8 { type Output = i8; fn sub(self, _rhs: &i8) -> i8 {} }
        impl Sub<i16> for i16 { type Output = i16; fn sub(self, _rhs: i16) -> i16 {} }
        impl Sub<&i16> for i16 { type Output = i16; fn sub(self, _rhs: &i16) -> i16 {} }
        impl Sub<i16> for &i16 { type Output = i16; fn sub(self, _rhs: i16) -> i16 {} }
        impl Sub<&i16> for &i16 { type Output = i16; fn sub(self, _rhs: &i16) -> i16 {} }
        impl Sub<i32> for i32 { type Output = i32; fn sub(self, _rhs: i32) -> i32 {} }
        impl Sub<&i32> for i32 { type Output = i32; fn sub(self, _rhs: &i32) -> i32 {} }
        impl Sub<i32> for &i32 { type Output = i32; fn sub(self, _rhs: i32) -> i32 {} }
        impl Sub<&i32> for &i32 { type Output = i32; fn sub(self, _rhs: &i32) -> i32 {} }
        impl Sub<i64> for i64 { type Output = i64; fn sub(self, _rhs: i64) -> i64 {} }
        impl Sub<&i64> for i64 { type Output = i64; fn sub(self, _rhs: &i64) -> i64 {} }
        impl Sub<i64> for &i64 { type Output = i64; fn sub(self, _rhs: i64) -> i64 {} }
        impl Sub<&i64> for &i64 { type Output = i64; fn sub(self, _rhs: &i64) -> i64 {} }
        impl Sub<i128> for i128 { type Output = i128; fn sub(self, _rhs: i128) -> i128 {} }
        impl Sub<&i128> for i128 { type Output = i128; fn sub(self, _rhs: &i128) -> i128 {} }
        impl Sub<i128> for &i128 { type Output = i128; fn sub(self, _rhs: i128) -> i128 {} }
        impl Sub<&i128> for &i128 { type Output = i128; fn sub(self, _rhs: &i128) -> i128 {} }
        impl Sub<isize> for isize { type Output = isize; fn sub(self, _rhs: isize) -> isize {} }
        impl Sub<&isize> for isize { type Output = isize; fn sub(self, _rhs: &isize) -> isize {} }
        impl Sub<isize> for &isize { type Output = isize; fn sub(self, _rhs: isize) -> isize {} }
        impl Sub<&isize> for &isize { type Output = isize; fn sub(self, _rhs: &isize) -> isize {} }
        impl Sub<u8> for u8 { type Output = u8; fn sub(self, _rhs: u8) -> u8 {} }
        impl Sub<&u8> for u8 { type Output = u8; fn sub(self, _rhs: &u8) -> u8 {} }
        impl Sub<u8> for &u8 { type Output = u8; fn sub(self, _rhs: u8) -> u8 {} }
        impl Sub<&u8> for &u8 { type Output = u8; fn sub(self, _rhs: &u8) -> u8 {} }
        impl Sub<u16> for u16 { type Output = u16; fn sub(self, _rhs: u16) -> u16 {} }
        impl Sub<&u16> for u16 { type Output = u16; fn sub(self, _rhs: &u16) -> u16 {} }
        impl Sub<u16> for &u16 { type Output = u16; fn sub(self, _rhs: u16) -> u16 {} }
        impl Sub<&u16> for &u16 { type Output = u16; fn sub(self, _rhs: &u16) -> u16 {} }
        impl Sub<u32> for u32 { type Output = u32; fn sub(self, _rhs: u32) -> u32 {} }
        impl Sub<&u32> for u32 { type Output = u32; fn sub(self, _rhs: &u32) -> u32 {} }
        impl Sub<u32> for &u32 { type Output = u32; fn sub(self, _rhs: u32) -> u32 {} }
        impl Sub<&u32> for &u32 { type Output = u32; fn sub(self, _rhs: &u32) -> u32 {} }
        impl Sub<u64> for u64 { type Output = u64; fn sub(self, _rhs: u64) -> u64 {} }
        impl Sub<&u64> for u64 { type Output = u64; fn sub(self, _rhs: &u64) -> u64 {} }
        impl Sub<u64> for &u64 { type Output = u64; fn sub(self, _rhs: u64) -> u64 {} }
        impl Sub<&u64> for &u64 { type Output = u64; fn sub(self, _rhs: &u64) -> u64 {} }
        impl Sub<u128> for u128 { type Output = u128; fn sub(self, _rhs: u128) -> u128 {} }
        impl Sub<&u128> for u128 { type Output = u128; fn sub(self, _rhs: &u128) -> u128 {} }
        impl Sub<u128> for &u128 { type Output = u128; fn sub(self, _rhs: u128) -> u128 {} }
        impl Sub<&u128> for &u128 { type Output = u128; fn sub(self, _rhs: &u128) -> u128 {} }
        impl Sub<usize> for usize { type Output = usize; fn sub(self, _rhs: usize) -> usize {} }
        impl Sub<&usize> for usize { type Output = usize; fn sub(self, _rhs: &usize) -> usize {} }
        impl Sub<usize> for &usize { type Output = usize; fn sub(self, _rhs: usize) -> usize {} }
        impl Sub<&usize> for &usize { type Output = usize; fn sub(self, _rhs: &usize) -> usize {} }
        impl Sub<f32> for f32 { type Output = f32; fn sub(self, _rhs: f32) -> f32 {} }
        impl Sub<&f32> for f32 { type Output = f32; fn sub(self, _rhs: &f32) -> f32 {} }
        impl Sub<f32> for &f32 { type Output = f32; fn sub(self, _rhs: f32) -> f32 {} }
        impl Sub<&f32> for &f32 { type Output = f32; fn sub(self, _rhs: &f32) -> f32 {} }
        impl Sub<f64> for f64 { type Output = f64; fn sub(self, _rhs: f64) -> f64 {} }
        impl Sub<&f64> for f64 { type Output = f64; fn sub(self, _rhs: &f64) -> f64 {} }
        impl Sub<f64> for &f64 { type Output = f64; fn sub(self, _rhs: f64) -> f64 {} }
        impl Sub<&f64> for &f64 { type Output = f64; fn sub(self, _rhs: &f64) -> f64 {} }

        pub trait Mul<Rhs = Self> { type Output; fn mul(self, rhs: Rhs) -> Self::Output; }
        impl Mul<i8> for i8 { type Output = i8; fn mul(self, _rhs: i8) -> i8 {} }
        impl Mul<&i8> for i8 { type Output = i8; fn mul(self, _rhs: &i8) -> i8 {} }
        impl Mul<i8> for &i8 { type Output = i8; fn mul(self, _rhs: i8) -> i8 {} }
        impl Mul<&i8> for &i8 { type Output = i8; fn mul(self, _rhs: &i8) -> i8 {} }
        impl Mul<i16> for i16 { type Output = i16; fn mul(self, _rhs: i16) -> i16 {} }
        impl Mul<&i16> for i16 { type Output = i16; fn mul(self, _rhs: &i16) -> i16 {} }
        impl Mul<i16> for &i16 { type Output = i16; fn mul(self, _rhs: i16) -> i16 {} }
        impl Mul<&i16> for &i16 { type Output = i16; fn mul(self, _rhs: &i16) -> i16 {} }
        impl Mul<i32> for i32 { type Output = i32; fn mul(self, _rhs: i32) -> i32 {} }
        impl Mul<&i32> for i32 { type Output = i32; fn mul(self, _rhs: &i32) -> i32 {} }
        impl Mul<i32> for &i32 { type Output = i32; fn mul(self, _rhs: i32) -> i32 {} }
        impl Mul<&i32> for &i32 { type Output = i32; fn mul(self, _rhs: &i32) -> i32 {} }
        impl Mul<i64> for i64 { type Output = i64; fn mul(self, _rhs: i64) -> i64 {} }
        impl Mul<&i64> for i64 { type Output = i64; fn mul(self, _rhs: &i64) -> i64 {} }
        impl Mul<i64> for &i64 { type Output = i64; fn mul(self, _rhs: i64) -> i64 {} }
        impl Mul<&i64> for &i64 { type Output = i64; fn mul(self, _rhs: &i64) -> i64 {} }
        impl Mul<i128> for i128 { type Output = i128; fn mul(self, _rhs: i128) -> i128 {} }
        impl Mul<&i128> for i128 { type Output = i128; fn mul(self, _rhs: &i128) -> i128 {} }
        impl Mul<i128> for &i128 { type Output = i128; fn mul(self, _rhs: i128) -> i128 {} }
        impl Mul<&i128> for &i128 { type Output = i128; fn mul(self, _rhs: &i128) -> i128 {} }
        impl Mul<isize> for isize { type Output = isize; fn mul(self, _rhs: isize) -> isize {} }
        impl Mul<&isize> for isize { type Output = isize; fn mul(self, _rhs: &isize) -> isize {} }
        impl Mul<isize> for &isize { type Output = isize; fn mul(self, _rhs: isize) -> isize {} }
        impl Mul<&isize> for &isize { type Output = isize; fn mul(self, _rhs: &isize) -> isize {} }
        impl Mul<u8> for u8 { type Output = u8; fn mul(self, _rhs: u8) -> u8 {} }
        impl Mul<&u8> for u8 { type Output = u8; fn mul(self, _rhs: &u8) -> u8 {} }
        impl Mul<u8> for &u8 { type Output = u8; fn mul(self, _rhs: u8) -> u8 {} }
        impl Mul<&u8> for &u8 { type Output = u8; fn mul(self, _rhs: &u8) -> u8 {} }
        impl Mul<u16> for u16 { type Output = u16; fn mul(self, _rhs: u16) -> u16 {} }
        impl Mul<&u16> for u16 { type Output = u16; fn mul(self, _rhs: &u16) -> u16 {} }
        impl Mul<u16> for &u16 { type Output = u16; fn mul(self, _rhs: u16) -> u16 {} }
        impl Mul<&u16> for &u16 { type Output = u16; fn mul(self, _rhs: &u16) -> u16 {} }
        impl Mul<u32> for u32 { type Output = u32; fn mul(self, _rhs: u32) -> u32 {} }
        impl Mul<&u32> for u32 { type Output = u32; fn mul(self, _rhs: &u32) -> u32 {} }
        impl Mul<u32> for &u32 { type Output = u32; fn mul(self, _rhs: u32) -> u32 {} }
        impl Mul<&u32> for &u32 { type Output = u32; fn mul(self, _rhs: &u32) -> u32 {} }
        impl Mul<u64> for u64 { type Output = u64; fn mul(self, _rhs: u64) -> u64 {} }
        impl Mul<&u64> for u64 { type Output = u64; fn mul(self, _rhs: &u64) -> u64 {} }
        impl Mul<u64> for &u64 { type Output = u64; fn mul(self, _rhs: u64) -> u64 {} }
        impl Mul<&u64> for &u64 { type Output = u64; fn mul(self, _rhs: &u64) -> u64 {} }
        impl Mul<u128> for u128 { type Output = u128; fn mul(self, _rhs: u128) -> u128 {} }
        impl Mul<&u128> for u128 { type Output = u128; fn mul(self, _rhs: &u128) -> u128 {} }
        impl Mul<u128> for &u128 { type Output = u128; fn mul(self, _rhs: u128) -> u128 {} }
        impl Mul<&u128> for &u128 { type Output = u128; fn mul(self, _rhs: &u128) -> u128 {} }
        impl Mul<usize> for usize { type Output = usize; fn mul(self, _rhs: usize) -> usize {} }
        impl Mul<&usize> for usize { type Output = usize; fn mul(self, _rhs: &usize) -> usize {} }
        impl Mul<usize> for &usize { type Output = usize; fn mul(self, _rhs: usize) -> usize {} }
        impl Mul<&usize> for &usize { type Output = usize; fn mul(self, _rhs: &usize) -> usize {} }
        impl Mul<f32> for f32 { type Output = f32; fn mul(self, _rhs: f32) -> f32 {} }
        impl Mul<&f32> for f32 { type Output = f32; fn mul(self, _rhs: &f32) -> f32 {} }
        impl Mul<f32> for &f32 { type Output = f32; fn mul(self, _rhs: f32) -> f32 {} }
        impl Mul<&f32> for &f32 { type Output = f32; fn mul(self, _rhs: &f32) -> f32 {} }
        impl Mul<f64> for f64 { type Output = f64; fn mul(self, _rhs: f64) -> f64 {} }
        impl Mul<&f64> for f64 { type Output = f64; fn mul(self, _rhs: &f64) -> f64 {} }
        impl Mul<f64> for &f64 { type Output = f64; fn mul(self, _rhs: f64) -> f64 {} }
        impl Mul<&f64> for &f64 { type Output = f64; fn mul(self, _rhs: &f64) -> f64 {} }

        pub trait Div<Rhs = Self> { type Output; fn div(self, rhs: Rhs) -> Self::Output; }
        impl Div<i8> for i8 { type Output = i8; fn div(self, _rhs: i8) -> i8 {} }
        impl Div<&i8> for i8 { type Output = i8; fn div(self, _rhs: &i8) -> i8 {} }
        impl Div<i8> for &i8 { type Output = i8; fn div(self, _rhs: i8) -> i8 {} }
        impl Div<&i8> for &i8 { type Output = i8; fn div(self, _rhs: &i8) -> i8 {} }
        impl Div<i16> for i16 { type Output = i16; fn div(self, _rhs: i16) -> i16 {} }
        impl Div<&i16> for i16 { type Output = i16; fn div(self, _rhs: &i16) -> i16 {} }
        impl Div<i16> for &i16 { type Output = i16; fn div(self, _rhs: i16) -> i16 {} }
        impl Div<&i16> for &i16 { type Output = i16; fn div(self, _rhs: &i16) -> i16 {} }
        impl Div<i32> for i32 { type Output = i32; fn div(self, _rhs: i32) -> i32 {} }
        impl Div<&i32> for i32 { type Output = i32; fn div(self, _rhs: &i32) -> i32 {} }
        impl Div<i32> for &i32 { type Output = i32; fn div(self, _rhs: i32) -> i32 {} }
        impl Div<&i32> for &i32 { type Output = i32; fn div(self, _rhs: &i32) -> i32 {} }
        impl Div<i64> for i64 { type Output = i64; fn div(self, _rhs: i64) -> i64 {} }
        impl Div<&i64> for i64 { type Output = i64; fn div(self, _rhs: &i64) -> i64 {} }
        impl Div<i64> for &i64 { type Output = i64; fn div(self, _rhs: i64) -> i64 {} }
        impl Div<&i64> for &i64 { type Output = i64; fn div(self, _rhs: &i64) -> i64 {} }
        impl Div<i128> for i128 { type Output = i128; fn div(self, _rhs: i128) -> i128 {} }
        impl Div<&i128> for i128 { type Output = i128; fn div(self, _rhs: &i128) -> i128 {} }
        impl Div<i128> for &i128 { type Output = i128; fn div(self, _rhs: i128) -> i128 {} }
        impl Div<&i128> for &i128 { type Output = i128; fn div(self, _rhs: &i128) -> i128 {} }
        impl Div<isize> for isize { type Output = isize; fn div(self, _rhs: isize) -> isize {} }
        impl Div<&isize> for isize { type Output = isize; fn div(self, _rhs: &isize) -> isize {} }
        impl Div<isize> for &isize { type Output = isize; fn div(self, _rhs: isize) -> isize {} }
        impl Div<&isize> for &isize { type Output = isize; fn div(self, _rhs: &isize) -> isize {} }
        impl Div<u8> for u8 { type Output = u8; fn div(self, _rhs: u8) -> u8 {} }
        impl Div<&u8> for u8 { type Output = u8; fn div(self, _rhs: &u8) -> u8 {} }
        impl Div<u8> for &u8 { type Output = u8; fn div(self, _rhs: u8) -> u8 {} }
        impl Div<&u8> for &u8 { type Output = u8; fn div(self, _rhs: &u8) -> u8 {} }
        impl Div<u16> for u16 { type Output = u16; fn div(self, _rhs: u16) -> u16 {} }
        impl Div<&u16> for u16 { type Output = u16; fn div(self, _rhs: &u16) -> u16 {} }
        impl Div<u16> for &u16 { type Output = u16; fn div(self, _rhs: u16) -> u16 {} }
        impl Div<&u16> for &u16 { type Output = u16; fn div(self, _rhs: &u16) -> u16 {} }
        impl Div<u32> for u32 { type Output = u32; fn div(self, _rhs: u32) -> u32 {} }
        impl Div<&u32> for u32 { type Output = u32; fn div(self, _rhs: &u32) -> u32 {} }
        impl Div<u32> for &u32 { type Output = u32; fn div(self, _rhs: u32) -> u32 {} }
        impl Div<&u32> for &u32 { type Output = u32; fn div(self, _rhs: &u32) -> u32 {} }
        impl Div<u64> for u64 { type Output = u64; fn div(self, _rhs: u64) -> u64 {} }
        impl Div<&u64> for u64 { type Output = u64; fn div(self, _rhs: &u64) -> u64 {} }
        impl Div<u64> for &u64 { type Output = u64; fn div(self, _rhs: u64) -> u64 {} }
        impl Div<&u64> for &u64 { type Output = u64; fn div(self, _rhs: &u64) -> u64 {} }
        impl Div<u128> for u128 { type Output = u128; fn div(self, _rhs: u128) -> u128 {} }
        impl Div<&u128> for u128 { type Output = u128; fn div(self, _rhs: &u128) -> u128 {} }
        impl Div<u128> for &u128 { type Output = u128; fn div(self, _rhs: u128) -> u128 {} }
        impl Div<&u128> for &u128 { type Output = u128; fn div(self, _rhs: &u128) -> u128 {} }
        impl Div<usize> for usize { type Output = usize; fn div(self, _rhs: usize) -> usize {} }
        impl Div<&usize> for usize { type Output = usize; fn div(self, _rhs: &usize) -> usize {} }
        impl Div<usize> for &usize { type Output = usize; fn div(self, _rhs: usize) -> usize {} }
        impl Div<&usize> for &usize { type Output = usize; fn div(self, _rhs: &usize) -> usize {} }
        impl Div<f32> for f32 { type Output = f32; fn div(self, _rhs: f32) -> f32 {} }
        impl Div<&f32> for f32 { type Output = f32; fn div(self, _rhs: &f32) -> f32 {} }
        impl Div<f32> for &f32 { type Output = f32; fn div(self, _rhs: f32) -> f32 {} }
        impl Div<&f32> for &f32 { type Output = f32; fn div(self, _rhs: &f32) -> f32 {} }
        impl Div<f64> for f64 { type Output = f64; fn div(self, _rhs: f64) -> f64 {} }
        impl Div<&f64> for f64 { type Output = f64; fn div(self, _rhs: &f64) -> f64 {} }
        impl Div<f64> for &f64 { type Output = f64; fn div(self, _rhs: f64) -> f64 {} }
        impl Div<&f64> for &f64 { type Output = f64; fn div(self, _rhs: &f64) -> f64 {} }

        pub trait Rem<Rhs = Self> { type Output; fn rem(self, rhs: Rhs) -> Self::Output; }
        impl Rem<i8> for i8 { type Output = i8; fn rem(self, _rhs: i8) -> i8 {} }
        impl Rem<&i8> for i8 { type Output = i8; fn rem(self, _rhs: &i8) -> i8 {} }
        impl Rem<i8> for &i8 { type Output = i8; fn rem(self, _rhs: i8) -> i8 {} }
        impl Rem<&i8> for &i8 { type Output = i8; fn rem(self, _rhs: &i8) -> i8 {} }
        impl Rem<i16> for i16 { type Output = i16; fn rem(self, _rhs: i16) -> i16 {} }
        impl Rem<&i16> for i16 { type Output = i16; fn rem(self, _rhs: &i16) -> i16 {} }
        impl Rem<i16> for &i16 { type Output = i16; fn rem(self, _rhs: i16) -> i16 {} }
        impl Rem<&i16> for &i16 { type Output = i16; fn rem(self, _rhs: &i16) -> i16 {} }
        impl Rem<i32> for i32 { type Output = i32; fn rem(self, _rhs: i32) -> i32 {} }
        impl Rem<&i32> for i32 { type Output = i32; fn rem(self, _rhs: &i32) -> i32 {} }
        impl Rem<i32> for &i32 { type Output = i32; fn rem(self, _rhs: i32) -> i32 {} }
        impl Rem<&i32> for &i32 { type Output = i32; fn rem(self, _rhs: &i32) -> i32 {} }
        impl Rem<i64> for i64 { type Output = i64; fn rem(self, _rhs: i64) -> i64 {} }
        impl Rem<&i64> for i64 { type Output = i64; fn rem(self, _rhs: &i64) -> i64 {} }
        impl Rem<i64> for &i64 { type Output = i64; fn rem(self, _rhs: i64) -> i64 {} }
        impl Rem<&i64> for &i64 { type Output = i64; fn rem(self, _rhs: &i64) -> i64 {} }
        impl Rem<i128> for i128 { type Output = i128; fn rem(self, _rhs: i128) -> i128 {} }
        impl Rem<&i128> for i128 { type Output = i128; fn rem(self, _rhs: &i128) -> i128 {} }
        impl Rem<i128> for &i128 { type Output = i128; fn rem(self, _rhs: i128) -> i128 {} }
        impl Rem<&i128> for &i128 { type Output = i128; fn rem(self, _rhs: &i128) -> i128 {} }
        impl Rem<isize> for isize { type Output = isize; fn rem(self, _rhs: isize) -> isize {} }
        impl Rem<&isize> for isize { type Output = isize; fn rem(self, _rhs: &isize) -> isize {} }
        impl Rem<isize> for &isize { type Output = isize; fn rem(self, _rhs: isize) -> isize {} }
        impl Rem<&isize> for &isize { type Output = isize; fn rem(self, _rhs: &isize) -> isize {} }
        impl Rem<u8> for u8 { type Output = u8; fn rem(self, _rhs: u8) -> u8 {} }
        impl Rem<&u8> for u8 { type Output = u8; fn rem(self, _rhs: &u8) -> u8 {} }
        impl Rem<u8> for &u8 { type Output = u8; fn rem(self, _rhs: u8) -> u8 {} }
        impl Rem<&u8> for &u8 { type Output = u8; fn rem(self, _rhs: &u8) -> u8 {} }
        impl Rem<u16> for u16 { type Output = u16; fn rem(self, _rhs: u16) -> u16 {} }
        impl Rem<&u16> for u16 { type Output = u16; fn rem(self, _rhs: &u16) -> u16 {} }
        impl Rem<u16> for &u16 { type Output = u16; fn rem(self, _rhs: u16) -> u16 {} }
        impl Rem<&u16> for &u16 { type Output = u16; fn rem(self, _rhs: &u16) -> u16 {} }
        impl Rem<u32> for u32 { type Output = u32; fn rem(self, _rhs: u32) -> u32 {} }
        impl Rem<&u32> for u32 { type Output = u32; fn rem(self, _rhs: &u32) -> u32 {} }
        impl Rem<u32> for &u32 { type Output = u32; fn rem(self, _rhs: u32) -> u32 {} }
        impl Rem<&u32> for &u32 { type Output = u32; fn rem(self, _rhs: &u32) -> u32 {} }
        impl Rem<u64> for u64 { type Output = u64; fn rem(self, _rhs: u64) -> u64 {} }
        impl Rem<&u64> for u64 { type Output = u64; fn rem(self, _rhs: &u64) -> u64 {} }
        impl Rem<u64> for &u64 { type Output = u64; fn rem(self, _rhs: u64) -> u64 {} }
        impl Rem<&u64> for &u64 { type Output = u64; fn rem(self, _rhs: &u64) -> u64 {} }
        impl Rem<u128> for u128 { type Output = u128; fn rem(self, _rhs: u128) -> u128 {} }
        impl Rem<&u128> for u128 { type Output = u128; fn rem(self, _rhs: &u128) -> u128 {} }
        impl Rem<u128> for &u128 { type Output = u128; fn rem(self, _rhs: u128) -> u128 {} }
        impl Rem<&u128> for &u128 { type Output = u128; fn rem(self, _rhs: &u128) -> u128 {} }
        impl Rem<usize> for usize { type Output = usize; fn rem(self, _rhs: usize) -> usize {} }
        impl Rem<&usize> for usize { type Output = usize; fn rem(self, _rhs: &usize) -> usize {} }
        impl Rem<usize> for &usize { type Output = usize; fn rem(self, _rhs: usize) -> usize {} }
        impl Rem<&usize> for &usize { type Output = usize; fn rem(self, _rhs: &usize) -> usize {} }
        impl Rem<f32> for f32 { type Output = f32; fn rem(self, _rhs: f32) -> f32 {} }
        impl Rem<&f32> for f32 { type Output = f32; fn rem(self, _rhs: &f32) -> f32 {} }
        impl Rem<f32> for &f32 { type Output = f32; fn rem(self, _rhs: f32) -> f32 {} }
        impl Rem<&f32> for &f32 { type Output = f32; fn rem(self, _rhs: &f32) -> f32 {} }
        impl Rem<f64> for f64 { type Output = f64; fn rem(self, _rhs: f64) -> f64 {} }
        impl Rem<&f64> for f64 { type Output = f64; fn rem(self, _rhs: &f64) -> f64 {} }
        impl Rem<f64> for &f64 { type Output = f64; fn rem(self, _rhs: f64) -> f64 {} }
        impl Rem<&f64> for &f64 { type Output = f64; fn rem(self, _rhs: &f64) -> f64 {} }

        pub trait BitXor<Rhs = Self> { type Output; fn bitxor(self, rhs: Rhs) -> Self::Output; }
        impl BitXor<i8> for i8 { type Output = i8; fn bitxor(self, _rhs: i8) -> i8 {} }
        impl BitXor<&i8> for i8 { type Output = i8; fn bitxor(self, _rhs: &i8) -> i8 {} }
        impl BitXor<i8> for &i8 { type Output = i8; fn bitxor(self, _rhs: i8) -> i8 {} }
        impl BitXor<&i8> for &i8 { type Output = i8; fn bitxor(self, _rhs: &i8) -> i8 {} }
        impl BitXor<i16> for i16 { type Output = i16; fn bitxor(self, _rhs: i16) -> i16 {} }
        impl BitXor<&i16> for i16 { type Output = i16; fn bitxor(self, _rhs: &i16) -> i16 {} }
        impl BitXor<i16> for &i16 { type Output = i16; fn bitxor(self, _rhs: i16) -> i16 {} }
        impl BitXor<&i16> for &i16 { type Output = i16; fn bitxor(self, _rhs: &i16) -> i16 {} }
        impl BitXor<i32> for i32 { type Output = i32; fn bitxor(self, _rhs: i32) -> i32 {} }
        impl BitXor<&i32> for i32 { type Output = i32; fn bitxor(self, _rhs: &i32) -> i32 {} }
        impl BitXor<i32> for &i32 { type Output = i32; fn bitxor(self, _rhs: i32) -> i32 {} }
        impl BitXor<&i32> for &i32 { type Output = i32; fn bitxor(self, _rhs: &i32) -> i32 {} }
        impl BitXor<i64> for i64 { type Output = i64; fn bitxor(self, _rhs: i64) -> i64 {} }
        impl BitXor<&i64> for i64 { type Output = i64; fn bitxor(self, _rhs: &i64) -> i64 {} }
        impl BitXor<i64> for &i64 { type Output = i64; fn bitxor(self, _rhs: i64) -> i64 {} }
        impl BitXor<&i64> for &i64 { type Output = i64; fn bitxor(self, _rhs: &i64) -> i64 {} }
        impl BitXor<i128> for i128 { type Output = i128; fn bitxor(self, _rhs: i128) -> i128 {} }
        impl BitXor<&i128> for i128 { type Output = i128; fn bitxor(self, _rhs: &i128) -> i128 {} }
        impl BitXor<i128> for &i128 { type Output = i128; fn bitxor(self, _rhs: i128) -> i128 {} }
        impl BitXor<&i128> for &i128 { type Output = i128; fn bitxor(self, _rhs: &i128) -> i128 {} }
        impl BitXor<isize> for isize { type Output = isize; fn bitxor(self, _rhs: isize) -> isize {} }
        impl BitXor<&isize> for isize { type Output = isize; fn bitxor(self, _rhs: &isize) -> isize {} }
        impl BitXor<isize> for &isize { type Output = isize; fn bitxor(self, _rhs: isize) -> isize {} }
        impl BitXor<&isize> for &isize { type Output = isize; fn bitxor(self, _rhs: &isize) -> isize {} }
        impl BitXor<u8> for u8 { type Output = u8; fn bitxor(self, _rhs: u8) -> u8 {} }
        impl BitXor<&u8> for u8 { type Output = u8; fn bitxor(self, _rhs: &u8) -> u8 {} }
        impl BitXor<u8> for &u8 { type Output = u8; fn bitxor(self, _rhs: u8) -> u8 {} }
        impl BitXor<&u8> for &u8 { type Output = u8; fn bitxor(self, _rhs: &u8) -> u8 {} }
        impl BitXor<u16> for u16 { type Output = u16; fn bitxor(self, _rhs: u16) -> u16 {} }
        impl BitXor<&u16> for u16 { type Output = u16; fn bitxor(self, _rhs: &u16) -> u16 {} }
        impl BitXor<u16> for &u16 { type Output = u16; fn bitxor(self, _rhs: u16) -> u16 {} }
        impl BitXor<&u16> for &u16 { type Output = u16; fn bitxor(self, _rhs: &u16) -> u16 {} }
        impl BitXor<u32> for u32 { type Output = u32; fn bitxor(self, _rhs: u32) -> u32 {} }
        impl BitXor<&u32> for u32 { type Output = u32; fn bitxor(self, _rhs: &u32) -> u32 {} }
        impl BitXor<u32> for &u32 { type Output = u32; fn bitxor(self, _rhs: u32) -> u32 {} }
        impl BitXor<&u32> for &u32 { type Output = u32; fn bitxor(self, _rhs: &u32) -> u32 {} }
        impl BitXor<u64> for u64 { type Output = u64; fn bitxor(self, _rhs: u64) -> u64 {} }
        impl BitXor<&u64> for u64 { type Output = u64; fn bitxor(self, _rhs: &u64) -> u64 {} }
        impl BitXor<u64> for &u64 { type Output = u64; fn bitxor(self, _rhs: u64) -> u64 {} }
        impl BitXor<&u64> for &u64 { type Output = u64; fn bitxor(self, _rhs: &u64) -> u64 {} }
        impl BitXor<u128> for u128 { type Output = u128; fn bitxor(self, _rhs: u128) -> u128 {} }
        impl BitXor<&u128> for u128 { type Output = u128; fn bitxor(self, _rhs: &u128) -> u128 {} }
        impl BitXor<u128> for &u128 { type Output = u128; fn bitxor(self, _rhs: u128) -> u128 {} }
        impl BitXor<&u128> for &u128 { type Output = u128; fn bitxor(self, _rhs: &u128) -> u128 {} }
        impl BitXor<usize> for usize { type Output = usize; fn bitxor(self, _rhs: usize) -> usize {} }
        impl BitXor<&usize> for usize { type Output = usize; fn bitxor(self, _rhs: &usize) -> usize {} }
        impl BitXor<usize> for &usize { type Output = usize; fn bitxor(self, _rhs: usize) -> usize {} }
        impl BitXor<&usize> for &usize { type Output = usize; fn bitxor(self, _rhs: &usize) -> usize {} }
        impl BitXor<bool> for bool { type Output = bool; fn bitxor(self, _rhs: bool) -> bool {} }
        impl BitXor<&bool> for bool { type Output = bool; fn bitxor(self, _rhs: &bool) -> bool {} }
        impl BitXor<bool> for &bool { type Output = bool; fn bitxor(self, _rhs: bool) -> bool {} }
        impl BitXor<&bool> for &bool { type Output = bool; fn bitxor(self, _rhs: &bool) -> bool {} }

        pub trait BitAnd<Rhs = Self> { type Output; fn bitand(self, rhs: Rhs) -> Self::Output; }
        impl BitAnd<i8> for i8 { type Output = i8; fn bitand(self, _rhs: i8) -> i8 {} }
        impl BitAnd<&i8> for i8 { type Output = i8; fn bitand(self, _rhs: &i8) -> i8 {} }
        impl BitAnd<i8> for &i8 { type Output = i8; fn bitand(self, _rhs: i8) -> i8 {} }
        impl BitAnd<&i8> for &i8 { type Output = i8; fn bitand(self, _rhs: &i8) -> i8 {} }
        impl BitAnd<i16> for i16 { type Output = i16; fn bitand(self, _rhs: i16) -> i16 {} }
        impl BitAnd<&i16> for i16 { type Output = i16; fn bitand(self, _rhs: &i16) -> i16 {} }
        impl BitAnd<i16> for &i16 { type Output = i16; fn bitand(self, _rhs: i16) -> i16 {} }
        impl BitAnd<&i16> for &i16 { type Output = i16; fn bitand(self, _rhs: &i16) -> i16 {} }
        impl BitAnd<i32> for i32 { type Output = i32; fn bitand(self, _rhs: i32) -> i32 {} }
        impl BitAnd<&i32> for i32 { type Output = i32; fn bitand(self, _rhs: &i32) -> i32 {} }
        impl BitAnd<i32> for &i32 { type Output = i32; fn bitand(self, _rhs: i32) -> i32 {} }
        impl BitAnd<&i32> for &i32 { type Output = i32; fn bitand(self, _rhs: &i32) -> i32 {} }
        impl BitAnd<i64> for i64 { type Output = i64; fn bitand(self, _rhs: i64) -> i64 {} }
        impl BitAnd<&i64> for i64 { type Output = i64; fn bitand(self, _rhs: &i64) -> i64 {} }
        impl BitAnd<i64> for &i64 { type Output = i64; fn bitand(self, _rhs: i64) -> i64 {} }
        impl BitAnd<&i64> for &i64 { type Output = i64; fn bitand(self, _rhs: &i64) -> i64 {} }
        impl BitAnd<i128> for i128 { type Output = i128; fn bitand(self, _rhs: i128) -> i128 {} }
        impl BitAnd<&i128> for i128 { type Output = i128; fn bitand(self, _rhs: &i128) -> i128 {} }
        impl BitAnd<i128> for &i128 { type Output = i128; fn bitand(self, _rhs: i128) -> i128 {} }
        impl BitAnd<&i128> for &i128 { type Output = i128; fn bitand(self, _rhs: &i128) -> i128 {} }
        impl BitAnd<isize> for isize { type Output = isize; fn bitand(self, _rhs: isize) -> isize {} }
        impl BitAnd<&isize> for isize { type Output = isize; fn bitand(self, _rhs: &isize) -> isize {} }
        impl BitAnd<isize> for &isize { type Output = isize; fn bitand(self, _rhs: isize) -> isize {} }
        impl BitAnd<&isize> for &isize { type Output = isize; fn bitand(self, _rhs: &isize) -> isize {} }
        impl BitAnd<u8> for u8 { type Output = u8; fn bitand(self, _rhs: u8) -> u8 {} }
        impl BitAnd<&u8> for u8 { type Output = u8; fn bitand(self, _rhs: &u8) -> u8 {} }
        impl BitAnd<u8> for &u8 { type Output = u8; fn bitand(self, _rhs: u8) -> u8 {} }
        impl BitAnd<&u8> for &u8 { type Output = u8; fn bitand(self, _rhs: &u8) -> u8 {} }
        impl BitAnd<u16> for u16 { type Output = u16; fn bitand(self, _rhs: u16) -> u16 {} }
        impl BitAnd<&u16> for u16 { type Output = u16; fn bitand(self, _rhs: &u16) -> u16 {} }
        impl BitAnd<u16> for &u16 { type Output = u16; fn bitand(self, _rhs: u16) -> u16 {} }
        impl BitAnd<&u16> for &u16 { type Output = u16; fn bitand(self, _rhs: &u16) -> u16 {} }
        impl BitAnd<u32> for u32 { type Output = u32; fn bitand(self, _rhs: u32) -> u32 {} }
        impl BitAnd<&u32> for u32 { type Output = u32; fn bitand(self, _rhs: &u32) -> u32 {} }
        impl BitAnd<u32> for &u32 { type Output = u32; fn bitand(self, _rhs: u32) -> u32 {} }
        impl BitAnd<&u32> for &u32 { type Output = u32; fn bitand(self, _rhs: &u32) -> u32 {} }
        impl BitAnd<u64> for u64 { type Output = u64; fn bitand(self, _rhs: u64) -> u64 {} }
        impl BitAnd<&u64> for u64 { type Output = u64; fn bitand(self, _rhs: &u64) -> u64 {} }
        impl BitAnd<u64> for &u64 { type Output = u64; fn bitand(self, _rhs: u64) -> u64 {} }
        impl BitAnd<&u64> for &u64 { type Output = u64; fn bitand(self, _rhs: &u64) -> u64 {} }
        impl BitAnd<u128> for u128 { type Output = u128; fn bitand(self, _rhs: u128) -> u128 {} }
        impl BitAnd<&u128> for u128 { type Output = u128; fn bitand(self, _rhs: &u128) -> u128 {} }
        impl BitAnd<u128> for &u128 { type Output = u128; fn bitand(self, _rhs: u128) -> u128 {} }
        impl BitAnd<&u128> for &u128 { type Output = u128; fn bitand(self, _rhs: &u128) -> u128 {} }
        impl BitAnd<usize> for usize { type Output = usize; fn bitand(self, _rhs: usize) -> usize {} }
        impl BitAnd<&usize> for usize { type Output = usize; fn bitand(self, _rhs: &usize) -> usize {} }
        impl BitAnd<usize> for &usize { type Output = usize; fn bitand(self, _rhs: usize) -> usize {} }
        impl BitAnd<&usize> for &usize { type Output = usize; fn bitand(self, _rhs: &usize) -> usize {} }
        impl BitAnd<bool> for bool { type Output = bool; fn bitand(self, _rhs: bool) -> bool {} }
        impl BitAnd<&bool> for bool { type Output = bool; fn bitand(self, _rhs: &bool) -> bool {} }
        impl BitAnd<bool> for &bool { type Output = bool; fn bitand(self, _rhs: bool) -> bool {} }
        impl BitAnd<&bool> for &bool { type Output = bool; fn bitand(self, _rhs: &bool) -> bool {} }

        pub trait BitOr<Rhs = Self> { type Output; fn bitor(self, rhs: Rhs) -> Self::Output; }
        impl BitOr<i8> for i8 { type Output = i8; fn bitor(self, _rhs: i8) -> i8 {} }
        impl BitOr<&i8> for i8 { type Output = i8; fn bitor(self, _rhs: &i8) -> i8 {} }
        impl BitOr<i8> for &i8 { type Output = i8; fn bitor(self, _rhs: i8) -> i8 {} }
        impl BitOr<&i8> for &i8 { type Output = i8; fn bitor(self, _rhs: &i8) -> i8 {} }
        impl BitOr<i16> for i16 { type Output = i16; fn bitor(self, _rhs: i16) -> i16 {} }
        impl BitOr<&i16> for i16 { type Output = i16; fn bitor(self, _rhs: &i16) -> i16 {} }
        impl BitOr<i16> for &i16 { type Output = i16; fn bitor(self, _rhs: i16) -> i16 {} }
        impl BitOr<&i16> for &i16 { type Output = i16; fn bitor(self, _rhs: &i16) -> i16 {} }
        impl BitOr<i32> for i32 { type Output = i32; fn bitor(self, _rhs: i32) -> i32 {} }
        impl BitOr<&i32> for i32 { type Output = i32; fn bitor(self, _rhs: &i32) -> i32 {} }
        impl BitOr<i32> for &i32 { type Output = i32; fn bitor(self, _rhs: i32) -> i32 {} }
        impl BitOr<&i32> for &i32 { type Output = i32; fn bitor(self, _rhs: &i32) -> i32 {} }
        impl BitOr<i64> for i64 { type Output = i64; fn bitor(self, _rhs: i64) -> i64 {} }
        impl BitOr<&i64> for i64 { type Output = i64; fn bitor(self, _rhs: &i64) -> i64 {} }
        impl BitOr<i64> for &i64 { type Output = i64; fn bitor(self, _rhs: i64) -> i64 {} }
        impl BitOr<&i64> for &i64 { type Output = i64; fn bitor(self, _rhs: &i64) -> i64 {} }
        impl BitOr<i128> for i128 { type Output = i128; fn bitor(self, _rhs: i128) -> i128 {} }
        impl BitOr<&i128> for i128 { type Output = i128; fn bitor(self, _rhs: &i128) -> i128 {} }
        impl BitOr<i128> for &i128 { type Output = i128; fn bitor(self, _rhs: i128) -> i128 {} }
        impl BitOr<&i128> for &i128 { type Output = i128; fn bitor(self, _rhs: &i128) -> i128 {} }
        impl BitOr<isize> for isize { type Output = isize; fn bitor(self, _rhs: isize) -> isize {} }
        impl BitOr<&isize> for isize { type Output = isize; fn bitor(self, _rhs: &isize) -> isize {} }
        impl BitOr<isize> for &isize { type Output = isize; fn bitor(self, _rhs: isize) -> isize {} }
        impl BitOr<&isize> for &isize { type Output = isize; fn bitor(self, _rhs: &isize) -> isize {} }
        impl BitOr<u8> for u8 { type Output = u8; fn bitor(self, _rhs: u8) -> u8 {} }
        impl BitOr<&u8> for u8 { type Output = u8; fn bitor(self, _rhs: &u8) -> u8 {} }
        impl BitOr<u8> for &u8 { type Output = u8; fn bitor(self, _rhs: u8) -> u8 {} }
        impl BitOr<&u8> for &u8 { type Output = u8; fn bitor(self, _rhs: &u8) -> u8 {} }
        impl BitOr<u16> for u16 { type Output = u16; fn bitor(self, _rhs: u16) -> u16 {} }
        impl BitOr<&u16> for u16 { type Output = u16; fn bitor(self, _rhs: &u16) -> u16 {} }
        impl BitOr<u16> for &u16 { type Output = u16; fn bitor(self, _rhs: u16) -> u16 {} }
        impl BitOr<&u16> for &u16 { type Output = u16; fn bitor(self, _rhs: &u16) -> u16 {} }
        impl BitOr<u32> for u32 { type Output = u32; fn bitor(self, _rhs: u32) -> u32 {} }
        impl BitOr<&u32> for u32 { type Output = u32; fn bitor(self, _rhs: &u32) -> u32 {} }
        impl BitOr<u32> for &u32 { type Output = u32; fn bitor(self, _rhs: u32) -> u32 {} }
        impl BitOr<&u32> for &u32 { type Output = u32; fn bitor(self, _rhs: &u32) -> u32 {} }
        impl BitOr<u64> for u64 { type Output = u64; fn bitor(self, _rhs: u64) -> u64 {} }
        impl BitOr<&u64> for u64 { type Output = u64; fn bitor(self, _rhs: &u64) -> u64 {} }
        impl BitOr<u64> for &u64 { type Output = u64; fn bitor(self, _rhs: u64) -> u64 {} }
        impl BitOr<&u64> for &u64 { type Output = u64; fn bitor(self, _rhs: &u64) -> u64 {} }
        impl BitOr<u128> for u128 { type Output = u128; fn bitor(self, _rhs: u128) -> u128 {} }
        impl BitOr<&u128> for u128 { type Output = u128; fn bitor(self, _rhs: &u128) -> u128 {} }
        impl BitOr<u128> for &u128 { type Output = u128; fn bitor(self, _rhs: u128) -> u128 {} }
        impl BitOr<&u128> for &u128 { type Output = u128; fn bitor(self, _rhs: &u128) -> u128 {} }
        impl BitOr<usize> for usize { type Output = usize; fn bitor(self, _rhs: usize) -> usize {} }
        impl BitOr<&usize> for usize { type Output = usize; fn bitor(self, _rhs: &usize) -> usize {} }
        impl BitOr<usize> for &usize { type Output = usize; fn bitor(self, _rhs: usize) -> usize {} }
        impl BitOr<&usize> for &usize { type Output = usize; fn bitor(self, _rhs: &usize) -> usize {} }
        impl BitOr<bool> for bool { type Output = bool; fn bitor(self, _rhs: bool) -> bool {} }
        impl BitOr<&bool> for bool { type Output = bool; fn bitor(self, _rhs: &bool) -> bool {} }
        impl BitOr<bool> for &bool { type Output = bool; fn bitor(self, _rhs: bool) -> bool {} }
        impl BitOr<&bool> for &bool { type Output = bool; fn bitor(self, _rhs: &bool) -> bool {} }

        // Shift operators

        pub trait Shl<Rhs = Self> { type Output; fn shl(self, rhs: Rhs) -> Self::Output; }
        impl Shl<i8> for i8 { type Output = i8; fn shl(self, _rhs: i8) -> i8 {} }
        impl Shl<&i8> for i8 { type Output = i8; fn shl(self, _rhs: &i8) -> i8 {} }
        impl Shl<i8> for &i8 { type Output = i8; fn shl(self, _rhs: i8) -> i8 {} }
        impl Shl<&i8> for &i8 { type Output = i8; fn shl(self, _rhs: &i8) -> i8 {} }
        impl Shl<i16> for i8 { type Output = i8; fn shl(self, _rhs: i16) -> i8 {} }
        impl Shl<&i16> for i8 { type Output = i8; fn shl(self, _rhs: &i16) -> i8 {} }
        impl Shl<i16> for &i8 { type Output = i8; fn shl(self, _rhs: i16) -> i8 {} }
        impl Shl<&i16> for &i8 { type Output = i8; fn shl(self, _rhs: &i16) -> i8 {} }
        impl Shl<i32> for i8 { type Output = i8; fn shl(self, _rhs: i32) -> i8 {} }
        impl Shl<&i32> for i8 { type Output = i8; fn shl(self, _rhs: &i32) -> i8 {} }
        impl Shl<i32> for &i8 { type Output = i8; fn shl(self, _rhs: i32) -> i8 {} }
        impl Shl<&i32> for &i8 { type Output = i8; fn shl(self, _rhs: &i32) -> i8 {} }
        impl Shl<i64> for i8 { type Output = i8; fn shl(self, _rhs: i64) -> i8 {} }
        impl Shl<&i64> for i8 { type Output = i8; fn shl(self, _rhs: &i64) -> i8 {} }
        impl Shl<i64> for &i8 { type Output = i8; fn shl(self, _rhs: i64) -> i8 {} }
        impl Shl<&i64> for &i8 { type Output = i8; fn shl(self, _rhs: &i64) -> i8 {} }
        impl Shl<i128> for i8 { type Output = i8; fn shl(self, _rhs: i128) -> i8 {} }
        impl Shl<&i128> for i8 { type Output = i8; fn shl(self, _rhs: &i128) -> i8 {} }
        impl Shl<i128> for &i8 { type Output = i8; fn shl(self, _rhs: i128) -> i8 {} }
        impl Shl<&i128> for &i8 { type Output = i8; fn shl(self, _rhs: &i128) -> i8 {} }
        impl Shl<isize> for i8 { type Output = i8; fn shl(self, _rhs: isize) -> i8 {} }
        impl Shl<&isize> for i8 { type Output = i8; fn shl(self, _rhs: &isize) -> i8 {} }
        impl Shl<isize> for &i8 { type Output = i8; fn shl(self, _rhs: isize) -> i8 {} }
        impl Shl<&isize> for &i8 { type Output = i8; fn shl(self, _rhs: &isize) -> i8 {} }
        impl Shl<u8> for i8 { type Output = i8; fn shl(self, _rhs: u8) -> i8 {} }
        impl Shl<&u8> for i8 { type Output = i8; fn shl(self, _rhs: &u8) -> i8 {} }
        impl Shl<u8> for &i8 { type Output = i8; fn shl(self, _rhs: u8) -> i8 {} }
        impl Shl<&u8> for &i8 { type Output = i8; fn shl(self, _rhs: &u8) -> i8 {} }
        impl Shl<u16> for i8 { type Output = i8; fn shl(self, _rhs: u16) -> i8 {} }
        impl Shl<&u16> for i8 { type Output = i8; fn shl(self, _rhs: &u16) -> i8 {} }
        impl Shl<u16> for &i8 { type Output = i8; fn shl(self, _rhs: u16) -> i8 {} }
        impl Shl<&u16> for &i8 { type Output = i8; fn shl(self, _rhs: &u16) -> i8 {} }
        impl Shl<u32> for i8 { type Output = i8; fn shl(self, _rhs: u32) -> i8 {} }
        impl Shl<&u32> for i8 { type Output = i8; fn shl(self, _rhs: &u32) -> i8 {} }
        impl Shl<u32> for &i8 { type Output = i8; fn shl(self, _rhs: u32) -> i8 {} }
        impl Shl<&u32> for &i8 { type Output = i8; fn shl(self, _rhs: &u32) -> i8 {} }
        impl Shl<u64> for i8 { type Output = i8; fn shl(self, _rhs: u64) -> i8 {} }
        impl Shl<&u64> for i8 { type Output = i8; fn shl(self, _rhs: &u64) -> i8 {} }
        impl Shl<u64> for &i8 { type Output = i8; fn shl(self, _rhs: u64) -> i8 {} }
        impl Shl<&u64> for &i8 { type Output = i8; fn shl(self, _rhs: &u64) -> i8 {} }
        impl Shl<u128> for i8 { type Output = i8; fn shl(self, _rhs: u128) -> i8 {} }
        impl Shl<&u128> for i8 { type Output = i8; fn shl(self, _rhs: &u128) -> i8 {} }
        impl Shl<u128> for &i8 { type Output = i8; fn shl(self, _rhs: u128) -> i8 {} }
        impl Shl<&u128> for &i8 { type Output = i8; fn shl(self, _rhs: &u128) -> i8 {} }
        impl Shl<usize> for i8 { type Output = i8; fn shl(self, _rhs: usize) -> i8 {} }
        impl Shl<&usize> for i8 { type Output = i8; fn shl(self, _rhs: &usize) -> i8 {} }
        impl Shl<usize> for &i8 { type Output = i8; fn shl(self, _rhs: usize) -> i8 {} }
        impl Shl<&usize> for &i8 { type Output = i8; fn shl(self, _rhs: &usize) -> i8 {} }
        impl Shl<i8> for i16 { type Output = i16; fn shl(self, _rhs: i8) -> i16 {} }
        impl Shl<&i8> for i16 { type Output = i16; fn shl(self, _rhs: &i8) -> i16 {} }
        impl Shl<i8> for &i16 { type Output = i16; fn shl(self, _rhs: i8) -> i16 {} }
        impl Shl<&i8> for &i16 { type Output = i16; fn shl(self, _rhs: &i8) -> i16 {} }
        impl Shl<i16> for i16 { type Output = i16; fn shl(self, _rhs: i16) -> i16 {} }
        impl Shl<&i16> for i16 { type Output = i16; fn shl(self, _rhs: &i16) -> i16 {} }
        impl Shl<i16> for &i16 { type Output = i16; fn shl(self, _rhs: i16) -> i16 {} }
        impl Shl<&i16> for &i16 { type Output = i16; fn shl(self, _rhs: &i16) -> i16 {} }
        impl Shl<i32> for i16 { type Output = i16; fn shl(self, _rhs: i32) -> i16 {} }
        impl Shl<&i32> for i16 { type Output = i16; fn shl(self, _rhs: &i32) -> i16 {} }
        impl Shl<i32> for &i16 { type Output = i16; fn shl(self, _rhs: i32) -> i16 {} }
        impl Shl<&i32> for &i16 { type Output = i16; fn shl(self, _rhs: &i32) -> i16 {} }
        impl Shl<i64> for i16 { type Output = i16; fn shl(self, _rhs: i64) -> i16 {} }
        impl Shl<&i64> for i16 { type Output = i16; fn shl(self, _rhs: &i64) -> i16 {} }
        impl Shl<i64> for &i16 { type Output = i16; fn shl(self, _rhs: i64) -> i16 {} }
        impl Shl<&i64> for &i16 { type Output = i16; fn shl(self, _rhs: &i64) -> i16 {} }
        impl Shl<i128> for i16 { type Output = i16; fn shl(self, _rhs: i128) -> i16 {} }
        impl Shl<&i128> for i16 { type Output = i16; fn shl(self, _rhs: &i128) -> i16 {} }
        impl Shl<i128> for &i16 { type Output = i16; fn shl(self, _rhs: i128) -> i16 {} }
        impl Shl<&i128> for &i16 { type Output = i16; fn shl(self, _rhs: &i128) -> i16 {} }
        impl Shl<isize> for i16 { type Output = i16; fn shl(self, _rhs: isize) -> i16 {} }
        impl Shl<&isize> for i16 { type Output = i16; fn shl(self, _rhs: &isize) -> i16 {} }
        impl Shl<isize> for &i16 { type Output = i16; fn shl(self, _rhs: isize) -> i16 {} }
        impl Shl<&isize> for &i16 { type Output = i16; fn shl(self, _rhs: &isize) -> i16 {} }
        impl Shl<u8> for i16 { type Output = i16; fn shl(self, _rhs: u8) -> i16 {} }
        impl Shl<&u8> for i16 { type Output = i16; fn shl(self, _rhs: &u8) -> i16 {} }
        impl Shl<u8> for &i16 { type Output = i16; fn shl(self, _rhs: u8) -> i16 {} }
        impl Shl<&u8> for &i16 { type Output = i16; fn shl(self, _rhs: &u8) -> i16 {} }
        impl Shl<u16> for i16 { type Output = i16; fn shl(self, _rhs: u16) -> i16 {} }
        impl Shl<&u16> for i16 { type Output = i16; fn shl(self, _rhs: &u16) -> i16 {} }
        impl Shl<u16> for &i16 { type Output = i16; fn shl(self, _rhs: u16) -> i16 {} }
        impl Shl<&u16> for &i16 { type Output = i16; fn shl(self, _rhs: &u16) -> i16 {} }
        impl Shl<u32> for i16 { type Output = i16; fn shl(self, _rhs: u32) -> i16 {} }
        impl Shl<&u32> for i16 { type Output = i16; fn shl(self, _rhs: &u32) -> i16 {} }
        impl Shl<u32> for &i16 { type Output = i16; fn shl(self, _rhs: u32) -> i16 {} }
        impl Shl<&u32> for &i16 { type Output = i16; fn shl(self, _rhs: &u32) -> i16 {} }
        impl Shl<u64> for i16 { type Output = i16; fn shl(self, _rhs: u64) -> i16 {} }
        impl Shl<&u64> for i16 { type Output = i16; fn shl(self, _rhs: &u64) -> i16 {} }
        impl Shl<u64> for &i16 { type Output = i16; fn shl(self, _rhs: u64) -> i16 {} }
        impl Shl<&u64> for &i16 { type Output = i16; fn shl(self, _rhs: &u64) -> i16 {} }
        impl Shl<u128> for i16 { type Output = i16; fn shl(self, _rhs: u128) -> i16 {} }
        impl Shl<&u128> for i16 { type Output = i16; fn shl(self, _rhs: &u128) -> i16 {} }
        impl Shl<u128> for &i16 { type Output = i16; fn shl(self, _rhs: u128) -> i16 {} }
        impl Shl<&u128> for &i16 { type Output = i16; fn shl(self, _rhs: &u128) -> i16 {} }
        impl Shl<usize> for i16 { type Output = i16; fn shl(self, _rhs: usize) -> i16 {} }
        impl Shl<&usize> for i16 { type Output = i16; fn shl(self, _rhs: &usize) -> i16 {} }
        impl Shl<usize> for &i16 { type Output = i16; fn shl(self, _rhs: usize) -> i16 {} }
        impl Shl<&usize> for &i16 { type Output = i16; fn shl(self, _rhs: &usize) -> i16 {} }
        impl Shl<i8> for i32 { type Output = i32; fn shl(self, _rhs: i8) -> i32 {} }
        impl Shl<&i8> for i32 { type Output = i32; fn shl(self, _rhs: &i8) -> i32 {} }
        impl Shl<i8> for &i32 { type Output = i32; fn shl(self, _rhs: i8) -> i32 {} }
        impl Shl<&i8> for &i32 { type Output = i32; fn shl(self, _rhs: &i8) -> i32 {} }
        impl Shl<i16> for i32 { type Output = i32; fn shl(self, _rhs: i16) -> i32 {} }
        impl Shl<&i16> for i32 { type Output = i32; fn shl(self, _rhs: &i16) -> i32 {} }
        impl Shl<i16> for &i32 { type Output = i32; fn shl(self, _rhs: i16) -> i32 {} }
        impl Shl<&i16> for &i32 { type Output = i32; fn shl(self, _rhs: &i16) -> i32 {} }
        impl Shl<i32> for i32 { type Output = i32; fn shl(self, _rhs: i32) -> i32 {} }
        impl Shl<&i32> for i32 { type Output = i32; fn shl(self, _rhs: &i32) -> i32 {} }
        impl Shl<i32> for &i32 { type Output = i32; fn shl(self, _rhs: i32) -> i32 {} }
        impl Shl<&i32> for &i32 { type Output = i32; fn shl(self, _rhs: &i32) -> i32 {} }
        impl Shl<i64> for i32 { type Output = i32; fn shl(self, _rhs: i64) -> i32 {} }
        impl Shl<&i64> for i32 { type Output = i32; fn shl(self, _rhs: &i64) -> i32 {} }
        impl Shl<i64> for &i32 { type Output = i32; fn shl(self, _rhs: i64) -> i32 {} }
        impl Shl<&i64> for &i32 { type Output = i32; fn shl(self, _rhs: &i64) -> i32 {} }
        impl Shl<i128> for i32 { type Output = i32; fn shl(self, _rhs: i128) -> i32 {} }
        impl Shl<&i128> for i32 { type Output = i32; fn shl(self, _rhs: &i128) -> i32 {} }
        impl Shl<i128> for &i32 { type Output = i32; fn shl(self, _rhs: i128) -> i32 {} }
        impl Shl<&i128> for &i32 { type Output = i32; fn shl(self, _rhs: &i128) -> i32 {} }
        impl Shl<isize> for i32 { type Output = i32; fn shl(self, _rhs: isize) -> i32 {} }
        impl Shl<&isize> for i32 { type Output = i32; fn shl(self, _rhs: &isize) -> i32 {} }
        impl Shl<isize> for &i32 { type Output = i32; fn shl(self, _rhs: isize) -> i32 {} }
        impl Shl<&isize> for &i32 { type Output = i32; fn shl(self, _rhs: &isize) -> i32 {} }
        impl Shl<u8> for i32 { type Output = i32; fn shl(self, _rhs: u8) -> i32 {} }
        impl Shl<&u8> for i32 { type Output = i32; fn shl(self, _rhs: &u8) -> i32 {} }
        impl Shl<u8> for &i32 { type Output = i32; fn shl(self, _rhs: u8) -> i32 {} }
        impl Shl<&u8> for &i32 { type Output = i32; fn shl(self, _rhs: &u8) -> i32 {} }
        impl Shl<u16> for i32 { type Output = i32; fn shl(self, _rhs: u16) -> i32 {} }
        impl Shl<&u16> for i32 { type Output = i32; fn shl(self, _rhs: &u16) -> i32 {} }
        impl Shl<u16> for &i32 { type Output = i32; fn shl(self, _rhs: u16) -> i32 {} }
        impl Shl<&u16> for &i32 { type Output = i32; fn shl(self, _rhs: &u16) -> i32 {} }
        impl Shl<u32> for i32 { type Output = i32; fn shl(self, _rhs: u32) -> i32 {} }
        impl Shl<&u32> for i32 { type Output = i32; fn shl(self, _rhs: &u32) -> i32 {} }
        impl Shl<u32> for &i32 { type Output = i32; fn shl(self, _rhs: u32) -> i32 {} }
        impl Shl<&u32> for &i32 { type Output = i32; fn shl(self, _rhs: &u32) -> i32 {} }
        impl Shl<u64> for i32 { type Output = i32; fn shl(self, _rhs: u64) -> i32 {} }
        impl Shl<&u64> for i32 { type Output = i32; fn shl(self, _rhs: &u64) -> i32 {} }
        impl Shl<u64> for &i32 { type Output = i32; fn shl(self, _rhs: u64) -> i32 {} }
        impl Shl<&u64> for &i32 { type Output = i32; fn shl(self, _rhs: &u64) -> i32 {} }
        impl Shl<u128> for i32 { type Output = i32; fn shl(self, _rhs: u128) -> i32 {} }
        impl Shl<&u128> for i32 { type Output = i32; fn shl(self, _rhs: &u128) -> i32 {} }
        impl Shl<u128> for &i32 { type Output = i32; fn shl(self, _rhs: u128) -> i32 {} }
        impl Shl<&u128> for &i32 { type Output = i32; fn shl(self, _rhs: &u128) -> i32 {} }
        impl Shl<usize> for i32 { type Output = i32; fn shl(self, _rhs: usize) -> i32 {} }
        impl Shl<&usize> for i32 { type Output = i32; fn shl(self, _rhs: &usize) -> i32 {} }
        impl Shl<usize> for &i32 { type Output = i32; fn shl(self, _rhs: usize) -> i32 {} }
        impl Shl<&usize> for &i32 { type Output = i32; fn shl(self, _rhs: &usize) -> i32 {} }
        impl Shl<i8> for i64 { type Output = i64; fn shl(self, _rhs: i8) -> i64 {} }
        impl Shl<&i8> for i64 { type Output = i64; fn shl(self, _rhs: &i8) -> i64 {} }
        impl Shl<i8> for &i64 { type Output = i64; fn shl(self, _rhs: i8) -> i64 {} }
        impl Shl<&i8> for &i64 { type Output = i64; fn shl(self, _rhs: &i8) -> i64 {} }
        impl Shl<i16> for i64 { type Output = i64; fn shl(self, _rhs: i16) -> i64 {} }
        impl Shl<&i16> for i64 { type Output = i64; fn shl(self, _rhs: &i16) -> i64 {} }
        impl Shl<i16> for &i64 { type Output = i64; fn shl(self, _rhs: i16) -> i64 {} }
        impl Shl<&i16> for &i64 { type Output = i64; fn shl(self, _rhs: &i16) -> i64 {} }
        impl Shl<i32> for i64 { type Output = i64; fn shl(self, _rhs: i32) -> i64 {} }
        impl Shl<&i32> for i64 { type Output = i64; fn shl(self, _rhs: &i32) -> i64 {} }
        impl Shl<i32> for &i64 { type Output = i64; fn shl(self, _rhs: i32) -> i64 {} }
        impl Shl<&i32> for &i64 { type Output = i64; fn shl(self, _rhs: &i32) -> i64 {} }
        impl Shl<i64> for i64 { type Output = i64; fn shl(self, _rhs: i64) -> i64 {} }
        impl Shl<&i64> for i64 { type Output = i64; fn shl(self, _rhs: &i64) -> i64 {} }
        impl Shl<i64> for &i64 { type Output = i64; fn shl(self, _rhs: i64) -> i64 {} }
        impl Shl<&i64> for &i64 { type Output = i64; fn shl(self, _rhs: &i64) -> i64 {} }
        impl Shl<i128> for i64 { type Output = i64; fn shl(self, _rhs: i128) -> i64 {} }
        impl Shl<&i128> for i64 { type Output = i64; fn shl(self, _rhs: &i128) -> i64 {} }
        impl Shl<i128> for &i64 { type Output = i64; fn shl(self, _rhs: i128) -> i64 {} }
        impl Shl<&i128> for &i64 { type Output = i64; fn shl(self, _rhs: &i128) -> i64 {} }
        impl Shl<isize> for i64 { type Output = i64; fn shl(self, _rhs: isize) -> i64 {} }
        impl Shl<&isize> for i64 { type Output = i64; fn shl(self, _rhs: &isize) -> i64 {} }
        impl Shl<isize> for &i64 { type Output = i64; fn shl(self, _rhs: isize) -> i64 {} }
        impl Shl<&isize> for &i64 { type Output = i64; fn shl(self, _rhs: &isize) -> i64 {} }
        impl Shl<u8> for i64 { type Output = i64; fn shl(self, _rhs: u8) -> i64 {} }
        impl Shl<&u8> for i64 { type Output = i64; fn shl(self, _rhs: &u8) -> i64 {} }
        impl Shl<u8> for &i64 { type Output = i64; fn shl(self, _rhs: u8) -> i64 {} }
        impl Shl<&u8> for &i64 { type Output = i64; fn shl(self, _rhs: &u8) -> i64 {} }
        impl Shl<u16> for i64 { type Output = i64; fn shl(self, _rhs: u16) -> i64 {} }
        impl Shl<&u16> for i64 { type Output = i64; fn shl(self, _rhs: &u16) -> i64 {} }
        impl Shl<u16> for &i64 { type Output = i64; fn shl(self, _rhs: u16) -> i64 {} }
        impl Shl<&u16> for &i64 { type Output = i64; fn shl(self, _rhs: &u16) -> i64 {} }
        impl Shl<u32> for i64 { type Output = i64; fn shl(self, _rhs: u32) -> i64 {} }
        impl Shl<&u32> for i64 { type Output = i64; fn shl(self, _rhs: &u32) -> i64 {} }
        impl Shl<u32> for &i64 { type Output = i64; fn shl(self, _rhs: u32) -> i64 {} }
        impl Shl<&u32> for &i64 { type Output = i64; fn shl(self, _rhs: &u32) -> i64 {} }
        impl Shl<u64> for i64 { type Output = i64; fn shl(self, _rhs: u64) -> i64 {} }
        impl Shl<&u64> for i64 { type Output = i64; fn shl(self, _rhs: &u64) -> i64 {} }
        impl Shl<u64> for &i64 { type Output = i64; fn shl(self, _rhs: u64) -> i64 {} }
        impl Shl<&u64> for &i64 { type Output = i64; fn shl(self, _rhs: &u64) -> i64 {} }
        impl Shl<u128> for i64 { type Output = i64; fn shl(self, _rhs: u128) -> i64 {} }
        impl Shl<&u128> for i64 { type Output = i64; fn shl(self, _rhs: &u128) -> i64 {} }
        impl Shl<u128> for &i64 { type Output = i64; fn shl(self, _rhs: u128) -> i64 {} }
        impl Shl<&u128> for &i64 { type Output = i64; fn shl(self, _rhs: &u128) -> i64 {} }
        impl Shl<usize> for i64 { type Output = i64; fn shl(self, _rhs: usize) -> i64 {} }
        impl Shl<&usize> for i64 { type Output = i64; fn shl(self, _rhs: &usize) -> i64 {} }
        impl Shl<usize> for &i64 { type Output = i64; fn shl(self, _rhs: usize) -> i64 {} }
        impl Shl<&usize> for &i64 { type Output = i64; fn shl(self, _rhs: &usize) -> i64 {} }
        impl Shl<i8> for i128 { type Output = i128; fn shl(self, _rhs: i8) -> i128 {} }
        impl Shl<&i8> for i128 { type Output = i128; fn shl(self, _rhs: &i8) -> i128 {} }
        impl Shl<i8> for &i128 { type Output = i128; fn shl(self, _rhs: i8) -> i128 {} }
        impl Shl<&i8> for &i128 { type Output = i128; fn shl(self, _rhs: &i8) -> i128 {} }
        impl Shl<i16> for i128 { type Output = i128; fn shl(self, _rhs: i16) -> i128 {} }
        impl Shl<&i16> for i128 { type Output = i128; fn shl(self, _rhs: &i16) -> i128 {} }
        impl Shl<i16> for &i128 { type Output = i128; fn shl(self, _rhs: i16) -> i128 {} }
        impl Shl<&i16> for &i128 { type Output = i128; fn shl(self, _rhs: &i16) -> i128 {} }
        impl Shl<i32> for i128 { type Output = i128; fn shl(self, _rhs: i32) -> i128 {} }
        impl Shl<&i32> for i128 { type Output = i128; fn shl(self, _rhs: &i32) -> i128 {} }
        impl Shl<i32> for &i128 { type Output = i128; fn shl(self, _rhs: i32) -> i128 {} }
        impl Shl<&i32> for &i128 { type Output = i128; fn shl(self, _rhs: &i32) -> i128 {} }
        impl Shl<i64> for i128 { type Output = i128; fn shl(self, _rhs: i64) -> i128 {} }
        impl Shl<&i64> for i128 { type Output = i128; fn shl(self, _rhs: &i64) -> i128 {} }
        impl Shl<i64> for &i128 { type Output = i128; fn shl(self, _rhs: i64) -> i128 {} }
        impl Shl<&i64> for &i128 { type Output = i128; fn shl(self, _rhs: &i64) -> i128 {} }
        impl Shl<i128> for i128 { type Output = i128; fn shl(self, _rhs: i128) -> i128 {} }
        impl Shl<&i128> for i128 { type Output = i128; fn shl(self, _rhs: &i128) -> i128 {} }
        impl Shl<i128> for &i128 { type Output = i128; fn shl(self, _rhs: i128) -> i128 {} }
        impl Shl<&i128> for &i128 { type Output = i128; fn shl(self, _rhs: &i128) -> i128 {} }
        impl Shl<isize> for i128 { type Output = i128; fn shl(self, _rhs: isize) -> i128 {} }
        impl Shl<&isize> for i128 { type Output = i128; fn shl(self, _rhs: &isize) -> i128 {} }
        impl Shl<isize> for &i128 { type Output = i128; fn shl(self, _rhs: isize) -> i128 {} }
        impl Shl<&isize> for &i128 { type Output = i128; fn shl(self, _rhs: &isize) -> i128 {} }
        impl Shl<u8> for i128 { type Output = i128; fn shl(self, _rhs: u8) -> i128 {} }
        impl Shl<&u8> for i128 { type Output = i128; fn shl(self, _rhs: &u8) -> i128 {} }
        impl Shl<u8> for &i128 { type Output = i128; fn shl(self, _rhs: u8) -> i128 {} }
        impl Shl<&u8> for &i128 { type Output = i128; fn shl(self, _rhs: &u8) -> i128 {} }
        impl Shl<u16> for i128 { type Output = i128; fn shl(self, _rhs: u16) -> i128 {} }
        impl Shl<&u16> for i128 { type Output = i128; fn shl(self, _rhs: &u16) -> i128 {} }
        impl Shl<u16> for &i128 { type Output = i128; fn shl(self, _rhs: u16) -> i128 {} }
        impl Shl<&u16> for &i128 { type Output = i128; fn shl(self, _rhs: &u16) -> i128 {} }
        impl Shl<u32> for i128 { type Output = i128; fn shl(self, _rhs: u32) -> i128 {} }
        impl Shl<&u32> for i128 { type Output = i128; fn shl(self, _rhs: &u32) -> i128 {} }
        impl Shl<u32> for &i128 { type Output = i128; fn shl(self, _rhs: u32) -> i128 {} }
        impl Shl<&u32> for &i128 { type Output = i128; fn shl(self, _rhs: &u32) -> i128 {} }
        impl Shl<u64> for i128 { type Output = i128; fn shl(self, _rhs: u64) -> i128 {} }
        impl Shl<&u64> for i128 { type Output = i128; fn shl(self, _rhs: &u64) -> i128 {} }
        impl Shl<u64> for &i128 { type Output = i128; fn shl(self, _rhs: u64) -> i128 {} }
        impl Shl<&u64> for &i128 { type Output = i128; fn shl(self, _rhs: &u64) -> i128 {} }
        impl Shl<u128> for i128 { type Output = i128; fn shl(self, _rhs: u128) -> i128 {} }
        impl Shl<&u128> for i128 { type Output = i128; fn shl(self, _rhs: &u128) -> i128 {} }
        impl Shl<u128> for &i128 { type Output = i128; fn shl(self, _rhs: u128) -> i128 {} }
        impl Shl<&u128> for &i128 { type Output = i128; fn shl(self, _rhs: &u128) -> i128 {} }
        impl Shl<usize> for i128 { type Output = i128; fn shl(self, _rhs: usize) -> i128 {} }
        impl Shl<&usize> for i128 { type Output = i128; fn shl(self, _rhs: &usize) -> i128 {} }
        impl Shl<usize> for &i128 { type Output = i128; fn shl(self, _rhs: usize) -> i128 {} }
        impl Shl<&usize> for &i128 { type Output = i128; fn shl(self, _rhs: &usize) -> i128 {} }
        impl Shl<i8> for isize { type Output = isize; fn shl(self, _rhs: i8) -> isize {} }
        impl Shl<&i8> for isize { type Output = isize; fn shl(self, _rhs: &i8) -> isize {} }
        impl Shl<i8> for &isize { type Output = isize; fn shl(self, _rhs: i8) -> isize {} }
        impl Shl<&i8> for &isize { type Output = isize; fn shl(self, _rhs: &i8) -> isize {} }
        impl Shl<i16> for isize { type Output = isize; fn shl(self, _rhs: i16) -> isize {} }
        impl Shl<&i16> for isize { type Output = isize; fn shl(self, _rhs: &i16) -> isize {} }
        impl Shl<i16> for &isize { type Output = isize; fn shl(self, _rhs: i16) -> isize {} }
        impl Shl<&i16> for &isize { type Output = isize; fn shl(self, _rhs: &i16) -> isize {} }
        impl Shl<i32> for isize { type Output = isize; fn shl(self, _rhs: i32) -> isize {} }
        impl Shl<&i32> for isize { type Output = isize; fn shl(self, _rhs: &i32) -> isize {} }
        impl Shl<i32> for &isize { type Output = isize; fn shl(self, _rhs: i32) -> isize {} }
        impl Shl<&i32> for &isize { type Output = isize; fn shl(self, _rhs: &i32) -> isize {} }
        impl Shl<i64> for isize { type Output = isize; fn shl(self, _rhs: i64) -> isize {} }
        impl Shl<&i64> for isize { type Output = isize; fn shl(self, _rhs: &i64) -> isize {} }
        impl Shl<i64> for &isize { type Output = isize; fn shl(self, _rhs: i64) -> isize {} }
        impl Shl<&i64> for &isize { type Output = isize; fn shl(self, _rhs: &i64) -> isize {} }
        impl Shl<i128> for isize { type Output = isize; fn shl(self, _rhs: i128) -> isize {} }
        impl Shl<&i128> for isize { type Output = isize; fn shl(self, _rhs: &i128) -> isize {} }
        impl Shl<i128> for &isize { type Output = isize; fn shl(self, _rhs: i128) -> isize {} }
        impl Shl<&i128> for &isize { type Output = isize; fn shl(self, _rhs: &i128) -> isize {} }
        impl Shl<isize> for isize { type Output = isize; fn shl(self, _rhs: isize) -> isize {} }
        impl Shl<&isize> for isize { type Output = isize; fn shl(self, _rhs: &isize) -> isize {} }
        impl Shl<isize> for &isize { type Output = isize; fn shl(self, _rhs: isize) -> isize {} }
        impl Shl<&isize> for &isize { type Output = isize; fn shl(self, _rhs: &isize) -> isize {} }
        impl Shl<u8> for isize { type Output = isize; fn shl(self, _rhs: u8) -> isize {} }
        impl Shl<&u8> for isize { type Output = isize; fn shl(self, _rhs: &u8) -> isize {} }
        impl Shl<u8> for &isize { type Output = isize; fn shl(self, _rhs: u8) -> isize {} }
        impl Shl<&u8> for &isize { type Output = isize; fn shl(self, _rhs: &u8) -> isize {} }
        impl Shl<u16> for isize { type Output = isize; fn shl(self, _rhs: u16) -> isize {} }
        impl Shl<&u16> for isize { type Output = isize; fn shl(self, _rhs: &u16) -> isize {} }
        impl Shl<u16> for &isize { type Output = isize; fn shl(self, _rhs: u16) -> isize {} }
        impl Shl<&u16> for &isize { type Output = isize; fn shl(self, _rhs: &u16) -> isize {} }
        impl Shl<u32> for isize { type Output = isize; fn shl(self, _rhs: u32) -> isize {} }
        impl Shl<&u32> for isize { type Output = isize; fn shl(self, _rhs: &u32) -> isize {} }
        impl Shl<u32> for &isize { type Output = isize; fn shl(self, _rhs: u32) -> isize {} }
        impl Shl<&u32> for &isize { type Output = isize; fn shl(self, _rhs: &u32) -> isize {} }
        impl Shl<u64> for isize { type Output = isize; fn shl(self, _rhs: u64) -> isize {} }
        impl Shl<&u64> for isize { type Output = isize; fn shl(self, _rhs: &u64) -> isize {} }
        impl Shl<u64> for &isize { type Output = isize; fn shl(self, _rhs: u64) -> isize {} }
        impl Shl<&u64> for &isize { type Output = isize; fn shl(self, _rhs: &u64) -> isize {} }
        impl Shl<u128> for isize { type Output = isize; fn shl(self, _rhs: u128) -> isize {} }
        impl Shl<&u128> for isize { type Output = isize; fn shl(self, _rhs: &u128) -> isize {} }
        impl Shl<u128> for &isize { type Output = isize; fn shl(self, _rhs: u128) -> isize {} }
        impl Shl<&u128> for &isize { type Output = isize; fn shl(self, _rhs: &u128) -> isize {} }
        impl Shl<usize> for isize { type Output = isize; fn shl(self, _rhs: usize) -> isize {} }
        impl Shl<&usize> for isize { type Output = isize; fn shl(self, _rhs: &usize) -> isize {} }
        impl Shl<usize> for &isize { type Output = isize; fn shl(self, _rhs: usize) -> isize {} }
        impl Shl<&usize> for &isize { type Output = isize; fn shl(self, _rhs: &usize) -> isize {} }
        impl Shl<i8> for u8 { type Output = u8; fn shl(self, _rhs: i8) -> u8 {} }
        impl Shl<&i8> for u8 { type Output = u8; fn shl(self, _rhs: &i8) -> u8 {} }
        impl Shl<i8> for &u8 { type Output = u8; fn shl(self, _rhs: i8) -> u8 {} }
        impl Shl<&i8> for &u8 { type Output = u8; fn shl(self, _rhs: &i8) -> u8 {} }
        impl Shl<i16> for u8 { type Output = u8; fn shl(self, _rhs: i16) -> u8 {} }
        impl Shl<&i16> for u8 { type Output = u8; fn shl(self, _rhs: &i16) -> u8 {} }
        impl Shl<i16> for &u8 { type Output = u8; fn shl(self, _rhs: i16) -> u8 {} }
        impl Shl<&i16> for &u8 { type Output = u8; fn shl(self, _rhs: &i16) -> u8 {} }
        impl Shl<i32> for u8 { type Output = u8; fn shl(self, _rhs: i32) -> u8 {} }
        impl Shl<&i32> for u8 { type Output = u8; fn shl(self, _rhs: &i32) -> u8 {} }
        impl Shl<i32> for &u8 { type Output = u8; fn shl(self, _rhs: i32) -> u8 {} }
        impl Shl<&i32> for &u8 { type Output = u8; fn shl(self, _rhs: &i32) -> u8 {} }
        impl Shl<i64> for u8 { type Output = u8; fn shl(self, _rhs: i64) -> u8 {} }
        impl Shl<&i64> for u8 { type Output = u8; fn shl(self, _rhs: &i64) -> u8 {} }
        impl Shl<i64> for &u8 { type Output = u8; fn shl(self, _rhs: i64) -> u8 {} }
        impl Shl<&i64> for &u8 { type Output = u8; fn shl(self, _rhs: &i64) -> u8 {} }
        impl Shl<i128> for u8 { type Output = u8; fn shl(self, _rhs: i128) -> u8 {} }
        impl Shl<&i128> for u8 { type Output = u8; fn shl(self, _rhs: &i128) -> u8 {} }
        impl Shl<i128> for &u8 { type Output = u8; fn shl(self, _rhs: i128) -> u8 {} }
        impl Shl<&i128> for &u8 { type Output = u8; fn shl(self, _rhs: &i128) -> u8 {} }
        impl Shl<isize> for u8 { type Output = u8; fn shl(self, _rhs: isize) -> u8 {} }
        impl Shl<&isize> for u8 { type Output = u8; fn shl(self, _rhs: &isize) -> u8 {} }
        impl Shl<isize> for &u8 { type Output = u8; fn shl(self, _rhs: isize) -> u8 {} }
        impl Shl<&isize> for &u8 { type Output = u8; fn shl(self, _rhs: &isize) -> u8 {} }
        impl Shl<u8> for u8 { type Output = u8; fn shl(self, _rhs: u8) -> u8 {} }
        impl Shl<&u8> for u8 { type Output = u8; fn shl(self, _rhs: &u8) -> u8 {} }
        impl Shl<u8> for &u8 { type Output = u8; fn shl(self, _rhs: u8) -> u8 {} }
        impl Shl<&u8> for &u8 { type Output = u8; fn shl(self, _rhs: &u8) -> u8 {} }
        impl Shl<u16> for u8 { type Output = u8; fn shl(self, _rhs: u16) -> u8 {} }
        impl Shl<&u16> for u8 { type Output = u8; fn shl(self, _rhs: &u16) -> u8 {} }
        impl Shl<u16> for &u8 { type Output = u8; fn shl(self, _rhs: u16) -> u8 {} }
        impl Shl<&u16> for &u8 { type Output = u8; fn shl(self, _rhs: &u16) -> u8 {} }
        impl Shl<u32> for u8 { type Output = u8; fn shl(self, _rhs: u32) -> u8 {} }
        impl Shl<&u32> for u8 { type Output = u8; fn shl(self, _rhs: &u32) -> u8 {} }
        impl Shl<u32> for &u8 { type Output = u8; fn shl(self, _rhs: u32) -> u8 {} }
        impl Shl<&u32> for &u8 { type Output = u8; fn shl(self, _rhs: &u32) -> u8 {} }
        impl Shl<u64> for u8 { type Output = u8; fn shl(self, _rhs: u64) -> u8 {} }
        impl Shl<&u64> for u8 { type Output = u8; fn shl(self, _rhs: &u64) -> u8 {} }
        impl Shl<u64> for &u8 { type Output = u8; fn shl(self, _rhs: u64) -> u8 {} }
        impl Shl<&u64> for &u8 { type Output = u8; fn shl(self, _rhs: &u64) -> u8 {} }
        impl Shl<u128> for u8 { type Output = u8; fn shl(self, _rhs: u128) -> u8 {} }
        impl Shl<&u128> for u8 { type Output = u8; fn shl(self, _rhs: &u128) -> u8 {} }
        impl Shl<u128> for &u8 { type Output = u8; fn shl(self, _rhs: u128) -> u8 {} }
        impl Shl<&u128> for &u8 { type Output = u8; fn shl(self, _rhs: &u128) -> u8 {} }
        impl Shl<usize> for u8 { type Output = u8; fn shl(self, _rhs: usize) -> u8 {} }
        impl Shl<&usize> for u8 { type Output = u8; fn shl(self, _rhs: &usize) -> u8 {} }
        impl Shl<usize> for &u8 { type Output = u8; fn shl(self, _rhs: usize) -> u8 {} }
        impl Shl<&usize> for &u8 { type Output = u8; fn shl(self, _rhs: &usize) -> u8 {} }
        impl Shl<i8> for u16 { type Output = u16; fn shl(self, _rhs: i8) -> u16 {} }
        impl Shl<&i8> for u16 { type Output = u16; fn shl(self, _rhs: &i8) -> u16 {} }
        impl Shl<i8> for &u16 { type Output = u16; fn shl(self, _rhs: i8) -> u16 {} }
        impl Shl<&i8> for &u16 { type Output = u16; fn shl(self, _rhs: &i8) -> u16 {} }
        impl Shl<i16> for u16 { type Output = u16; fn shl(self, _rhs: i16) -> u16 {} }
        impl Shl<&i16> for u16 { type Output = u16; fn shl(self, _rhs: &i16) -> u16 {} }
        impl Shl<i16> for &u16 { type Output = u16; fn shl(self, _rhs: i16) -> u16 {} }
        impl Shl<&i16> for &u16 { type Output = u16; fn shl(self, _rhs: &i16) -> u16 {} }
        impl Shl<i32> for u16 { type Output = u16; fn shl(self, _rhs: i32) -> u16 {} }
        impl Shl<&i32> for u16 { type Output = u16; fn shl(self, _rhs: &i32) -> u16 {} }
        impl Shl<i32> for &u16 { type Output = u16; fn shl(self, _rhs: i32) -> u16 {} }
        impl Shl<&i32> for &u16 { type Output = u16; fn shl(self, _rhs: &i32) -> u16 {} }
        impl Shl<i64> for u16 { type Output = u16; fn shl(self, _rhs: i64) -> u16 {} }
        impl Shl<&i64> for u16 { type Output = u16; fn shl(self, _rhs: &i64) -> u16 {} }
        impl Shl<i64> for &u16 { type Output = u16; fn shl(self, _rhs: i64) -> u16 {} }
        impl Shl<&i64> for &u16 { type Output = u16; fn shl(self, _rhs: &i64) -> u16 {} }
        impl Shl<i128> for u16 { type Output = u16; fn shl(self, _rhs: i128) -> u16 {} }
        impl Shl<&i128> for u16 { type Output = u16; fn shl(self, _rhs: &i128) -> u16 {} }
        impl Shl<i128> for &u16 { type Output = u16; fn shl(self, _rhs: i128) -> u16 {} }
        impl Shl<&i128> for &u16 { type Output = u16; fn shl(self, _rhs: &i128) -> u16 {} }
        impl Shl<isize> for u16 { type Output = u16; fn shl(self, _rhs: isize) -> u16 {} }
        impl Shl<&isize> for u16 { type Output = u16; fn shl(self, _rhs: &isize) -> u16 {} }
        impl Shl<isize> for &u16 { type Output = u16; fn shl(self, _rhs: isize) -> u16 {} }
        impl Shl<&isize> for &u16 { type Output = u16; fn shl(self, _rhs: &isize) -> u16 {} }
        impl Shl<u8> for u16 { type Output = u16; fn shl(self, _rhs: u8) -> u16 {} }
        impl Shl<&u8> for u16 { type Output = u16; fn shl(self, _rhs: &u8) -> u16 {} }
        impl Shl<u8> for &u16 { type Output = u16; fn shl(self, _rhs: u8) -> u16 {} }
        impl Shl<&u8> for &u16 { type Output = u16; fn shl(self, _rhs: &u8) -> u16 {} }
        impl Shl<u16> for u16 { type Output = u16; fn shl(self, _rhs: u16) -> u16 {} }
        impl Shl<&u16> for u16 { type Output = u16; fn shl(self, _rhs: &u16) -> u16 {} }
        impl Shl<u16> for &u16 { type Output = u16; fn shl(self, _rhs: u16) -> u16 {} }
        impl Shl<&u16> for &u16 { type Output = u16; fn shl(self, _rhs: &u16) -> u16 {} }
        impl Shl<u32> for u16 { type Output = u16; fn shl(self, _rhs: u32) -> u16 {} }
        impl Shl<&u32> for u16 { type Output = u16; fn shl(self, _rhs: &u32) -> u16 {} }
        impl Shl<u32> for &u16 { type Output = u16; fn shl(self, _rhs: u32) -> u16 {} }
        impl Shl<&u32> for &u16 { type Output = u16; fn shl(self, _rhs: &u32) -> u16 {} }
        impl Shl<u64> for u16 { type Output = u16; fn shl(self, _rhs: u64) -> u16 {} }
        impl Shl<&u64> for u16 { type Output = u16; fn shl(self, _rhs: &u64) -> u16 {} }
        impl Shl<u64> for &u16 { type Output = u16; fn shl(self, _rhs: u64) -> u16 {} }
        impl Shl<&u64> for &u16 { type Output = u16; fn shl(self, _rhs: &u64) -> u16 {} }
        impl Shl<u128> for u16 { type Output = u16; fn shl(self, _rhs: u128) -> u16 {} }
        impl Shl<&u128> for u16 { type Output = u16; fn shl(self, _rhs: &u128) -> u16 {} }
        impl Shl<u128> for &u16 { type Output = u16; fn shl(self, _rhs: u128) -> u16 {} }
        impl Shl<&u128> for &u16 { type Output = u16; fn shl(self, _rhs: &u128) -> u16 {} }
        impl Shl<usize> for u16 { type Output = u16; fn shl(self, _rhs: usize) -> u16 {} }
        impl Shl<&usize> for u16 { type Output = u16; fn shl(self, _rhs: &usize) -> u16 {} }
        impl Shl<usize> for &u16 { type Output = u16; fn shl(self, _rhs: usize) -> u16 {} }
        impl Shl<&usize> for &u16 { type Output = u16; fn shl(self, _rhs: &usize) -> u16 {} }
        impl Shl<i8> for u32 { type Output = u32; fn shl(self, _rhs: i8) -> u32 {} }
        impl Shl<&i8> for u32 { type Output = u32; fn shl(self, _rhs: &i8) -> u32 {} }
        impl Shl<i8> for &u32 { type Output = u32; fn shl(self, _rhs: i8) -> u32 {} }
        impl Shl<&i8> for &u32 { type Output = u32; fn shl(self, _rhs: &i8) -> u32 {} }
        impl Shl<i16> for u32 { type Output = u32; fn shl(self, _rhs: i16) -> u32 {} }
        impl Shl<&i16> for u32 { type Output = u32; fn shl(self, _rhs: &i16) -> u32 {} }
        impl Shl<i16> for &u32 { type Output = u32; fn shl(self, _rhs: i16) -> u32 {} }
        impl Shl<&i16> for &u32 { type Output = u32; fn shl(self, _rhs: &i16) -> u32 {} }
        impl Shl<i32> for u32 { type Output = u32; fn shl(self, _rhs: i32) -> u32 {} }
        impl Shl<&i32> for u32 { type Output = u32; fn shl(self, _rhs: &i32) -> u32 {} }
        impl Shl<i32> for &u32 { type Output = u32; fn shl(self, _rhs: i32) -> u32 {} }
        impl Shl<&i32> for &u32 { type Output = u32; fn shl(self, _rhs: &i32) -> u32 {} }
        impl Shl<i64> for u32 { type Output = u32; fn shl(self, _rhs: i64) -> u32 {} }
        impl Shl<&i64> for u32 { type Output = u32; fn shl(self, _rhs: &i64) -> u32 {} }
        impl Shl<i64> for &u32 { type Output = u32; fn shl(self, _rhs: i64) -> u32 {} }
        impl Shl<&i64> for &u32 { type Output = u32; fn shl(self, _rhs: &i64) -> u32 {} }
        impl Shl<i128> for u32 { type Output = u32; fn shl(self, _rhs: i128) -> u32 {} }
        impl Shl<&i128> for u32 { type Output = u32; fn shl(self, _rhs: &i128) -> u32 {} }
        impl Shl<i128> for &u32 { type Output = u32; fn shl(self, _rhs: i128) -> u32 {} }
        impl Shl<&i128> for &u32 { type Output = u32; fn shl(self, _rhs: &i128) -> u32 {} }
        impl Shl<isize> for u32 { type Output = u32; fn shl(self, _rhs: isize) -> u32 {} }
        impl Shl<&isize> for u32 { type Output = u32; fn shl(self, _rhs: &isize) -> u32 {} }
        impl Shl<isize> for &u32 { type Output = u32; fn shl(self, _rhs: isize) -> u32 {} }
        impl Shl<&isize> for &u32 { type Output = u32; fn shl(self, _rhs: &isize) -> u32 {} }
        impl Shl<u8> for u32 { type Output = u32; fn shl(self, _rhs: u8) -> u32 {} }
        impl Shl<&u8> for u32 { type Output = u32; fn shl(self, _rhs: &u8) -> u32 {} }
        impl Shl<u8> for &u32 { type Output = u32; fn shl(self, _rhs: u8) -> u32 {} }
        impl Shl<&u8> for &u32 { type Output = u32; fn shl(self, _rhs: &u8) -> u32 {} }
        impl Shl<u16> for u32 { type Output = u32; fn shl(self, _rhs: u16) -> u32 {} }
        impl Shl<&u16> for u32 { type Output = u32; fn shl(self, _rhs: &u16) -> u32 {} }
        impl Shl<u16> for &u32 { type Output = u32; fn shl(self, _rhs: u16) -> u32 {} }
        impl Shl<&u16> for &u32 { type Output = u32; fn shl(self, _rhs: &u16) -> u32 {} }
        impl Shl<u32> for u32 { type Output = u32; fn shl(self, _rhs: u32) -> u32 {} }
        impl Shl<&u32> for u32 { type Output = u32; fn shl(self, _rhs: &u32) -> u32 {} }
        impl Shl<u32> for &u32 { type Output = u32; fn shl(self, _rhs: u32) -> u32 {} }
        impl Shl<&u32> for &u32 { type Output = u32; fn shl(self, _rhs: &u32) -> u32 {} }
        impl Shl<u64> for u32 { type Output = u32; fn shl(self, _rhs: u64) -> u32 {} }
        impl Shl<&u64> for u32 { type Output = u32; fn shl(self, _rhs: &u64) -> u32 {} }
        impl Shl<u64> for &u32 { type Output = u32; fn shl(self, _rhs: u64) -> u32 {} }
        impl Shl<&u64> for &u32 { type Output = u32; fn shl(self, _rhs: &u64) -> u32 {} }
        impl Shl<u128> for u32 { type Output = u32; fn shl(self, _rhs: u128) -> u32 {} }
        impl Shl<&u128> for u32 { type Output = u32; fn shl(self, _rhs: &u128) -> u32 {} }
        impl Shl<u128> for &u32 { type Output = u32; fn shl(self, _rhs: u128) -> u32 {} }
        impl Shl<&u128> for &u32 { type Output = u32; fn shl(self, _rhs: &u128) -> u32 {} }
        impl Shl<usize> for u32 { type Output = u32; fn shl(self, _rhs: usize) -> u32 {} }
        impl Shl<&usize> for u32 { type Output = u32; fn shl(self, _rhs: &usize) -> u32 {} }
        impl Shl<usize> for &u32 { type Output = u32; fn shl(self, _rhs: usize) -> u32 {} }
        impl Shl<&usize> for &u32 { type Output = u32; fn shl(self, _rhs: &usize) -> u32 {} }
        impl Shl<i8> for u64 { type Output = u64; fn shl(self, _rhs: i8) -> u64 {} }
        impl Shl<&i8> for u64 { type Output = u64; fn shl(self, _rhs: &i8) -> u64 {} }
        impl Shl<i8> for &u64 { type Output = u64; fn shl(self, _rhs: i8) -> u64 {} }
        impl Shl<&i8> for &u64 { type Output = u64; fn shl(self, _rhs: &i8) -> u64 {} }
        impl Shl<i16> for u64 { type Output = u64; fn shl(self, _rhs: i16) -> u64 {} }
        impl Shl<&i16> for u64 { type Output = u64; fn shl(self, _rhs: &i16) -> u64 {} }
        impl Shl<i16> for &u64 { type Output = u64; fn shl(self, _rhs: i16) -> u64 {} }
        impl Shl<&i16> for &u64 { type Output = u64; fn shl(self, _rhs: &i16) -> u64 {} }
        impl Shl<i32> for u64 { type Output = u64; fn shl(self, _rhs: i32) -> u64 {} }
        impl Shl<&i32> for u64 { type Output = u64; fn shl(self, _rhs: &i32) -> u64 {} }
        impl Shl<i32> for &u64 { type Output = u64; fn shl(self, _rhs: i32) -> u64 {} }
        impl Shl<&i32> for &u64 { type Output = u64; fn shl(self, _rhs: &i32) -> u64 {} }
        impl Shl<i64> for u64 { type Output = u64; fn shl(self, _rhs: i64) -> u64 {} }
        impl Shl<&i64> for u64 { type Output = u64; fn shl(self, _rhs: &i64) -> u64 {} }
        impl Shl<i64> for &u64 { type Output = u64; fn shl(self, _rhs: i64) -> u64 {} }
        impl Shl<&i64> for &u64 { type Output = u64; fn shl(self, _rhs: &i64) -> u64 {} }
        impl Shl<i128> for u64 { type Output = u64; fn shl(self, _rhs: i128) -> u64 {} }
        impl Shl<&i128> for u64 { type Output = u64; fn shl(self, _rhs: &i128) -> u64 {} }
        impl Shl<i128> for &u64 { type Output = u64; fn shl(self, _rhs: i128) -> u64 {} }
        impl Shl<&i128> for &u64 { type Output = u64; fn shl(self, _rhs: &i128) -> u64 {} }
        impl Shl<isize> for u64 { type Output = u64; fn shl(self, _rhs: isize) -> u64 {} }
        impl Shl<&isize> for u64 { type Output = u64; fn shl(self, _rhs: &isize) -> u64 {} }
        impl Shl<isize> for &u64 { type Output = u64; fn shl(self, _rhs: isize) -> u64 {} }
        impl Shl<&isize> for &u64 { type Output = u64; fn shl(self, _rhs: &isize) -> u64 {} }
        impl Shl<u8> for u64 { type Output = u64; fn shl(self, _rhs: u8) -> u64 {} }
        impl Shl<&u8> for u64 { type Output = u64; fn shl(self, _rhs: &u8) -> u64 {} }
        impl Shl<u8> for &u64 { type Output = u64; fn shl(self, _rhs: u8) -> u64 {} }
        impl Shl<&u8> for &u64 { type Output = u64; fn shl(self, _rhs: &u8) -> u64 {} }
        impl Shl<u16> for u64 { type Output = u64; fn shl(self, _rhs: u16) -> u64 {} }
        impl Shl<&u16> for u64 { type Output = u64; fn shl(self, _rhs: &u16) -> u64 {} }
        impl Shl<u16> for &u64 { type Output = u64; fn shl(self, _rhs: u16) -> u64 {} }
        impl Shl<&u16> for &u64 { type Output = u64; fn shl(self, _rhs: &u16) -> u64 {} }
        impl Shl<u32> for u64 { type Output = u64; fn shl(self, _rhs: u32) -> u64 {} }
        impl Shl<&u32> for u64 { type Output = u64; fn shl(self, _rhs: &u32) -> u64 {} }
        impl Shl<u32> for &u64 { type Output = u64; fn shl(self, _rhs: u32) -> u64 {} }
        impl Shl<&u32> for &u64 { type Output = u64; fn shl(self, _rhs: &u32) -> u64 {} }
        impl Shl<u64> for u64 { type Output = u64; fn shl(self, _rhs: u64) -> u64 {} }
        impl Shl<&u64> for u64 { type Output = u64; fn shl(self, _rhs: &u64) -> u64 {} }
        impl Shl<u64> for &u64 { type Output = u64; fn shl(self, _rhs: u64) -> u64 {} }
        impl Shl<&u64> for &u64 { type Output = u64; fn shl(self, _rhs: &u64) -> u64 {} }
        impl Shl<u128> for u64 { type Output = u64; fn shl(self, _rhs: u128) -> u64 {} }
        impl Shl<&u128> for u64 { type Output = u64; fn shl(self, _rhs: &u128) -> u64 {} }
        impl Shl<u128> for &u64 { type Output = u64; fn shl(self, _rhs: u128) -> u64 {} }
        impl Shl<&u128> for &u64 { type Output = u64; fn shl(self, _rhs: &u128) -> u64 {} }
        impl Shl<usize> for u64 { type Output = u64; fn shl(self, _rhs: usize) -> u64 {} }
        impl Shl<&usize> for u64 { type Output = u64; fn shl(self, _rhs: &usize) -> u64 {} }
        impl Shl<usize> for &u64 { type Output = u64; fn shl(self, _rhs: usize) -> u64 {} }
        impl Shl<&usize> for &u64 { type Output = u64; fn shl(self, _rhs: &usize) -> u64 {} }
        impl Shl<i8> for u128 { type Output = u128; fn shl(self, _rhs: i8) -> u128 {} }
        impl Shl<&i8> for u128 { type Output = u128; fn shl(self, _rhs: &i8) -> u128 {} }
        impl Shl<i8> for &u128 { type Output = u128; fn shl(self, _rhs: i8) -> u128 {} }
        impl Shl<&i8> for &u128 { type Output = u128; fn shl(self, _rhs: &i8) -> u128 {} }
        impl Shl<i16> for u128 { type Output = u128; fn shl(self, _rhs: i16) -> u128 {} }
        impl Shl<&i16> for u128 { type Output = u128; fn shl(self, _rhs: &i16) -> u128 {} }
        impl Shl<i16> for &u128 { type Output = u128; fn shl(self, _rhs: i16) -> u128 {} }
        impl Shl<&i16> for &u128 { type Output = u128; fn shl(self, _rhs: &i16) -> u128 {} }
        impl Shl<i32> for u128 { type Output = u128; fn shl(self, _rhs: i32) -> u128 {} }
        impl Shl<&i32> for u128 { type Output = u128; fn shl(self, _rhs: &i32) -> u128 {} }
        impl Shl<i32> for &u128 { type Output = u128; fn shl(self, _rhs: i32) -> u128 {} }
        impl Shl<&i32> for &u128 { type Output = u128; fn shl(self, _rhs: &i32) -> u128 {} }
        impl Shl<i64> for u128 { type Output = u128; fn shl(self, _rhs: i64) -> u128 {} }
        impl Shl<&i64> for u128 { type Output = u128; fn shl(self, _rhs: &i64) -> u128 {} }
        impl Shl<i64> for &u128 { type Output = u128; fn shl(self, _rhs: i64) -> u128 {} }
        impl Shl<&i64> for &u128 { type Output = u128; fn shl(self, _rhs: &i64) -> u128 {} }
        impl Shl<i128> for u128 { type Output = u128; fn shl(self, _rhs: i128) -> u128 {} }
        impl Shl<&i128> for u128 { type Output = u128; fn shl(self, _rhs: &i128) -> u128 {} }
        impl Shl<i128> for &u128 { type Output = u128; fn shl(self, _rhs: i128) -> u128 {} }
        impl Shl<&i128> for &u128 { type Output = u128; fn shl(self, _rhs: &i128) -> u128 {} }
        impl Shl<isize> for u128 { type Output = u128; fn shl(self, _rhs: isize) -> u128 {} }
        impl Shl<&isize> for u128 { type Output = u128; fn shl(self, _rhs: &isize) -> u128 {} }
        impl Shl<isize> for &u128 { type Output = u128; fn shl(self, _rhs: isize) -> u128 {} }
        impl Shl<&isize> for &u128 { type Output = u128; fn shl(self, _rhs: &isize) -> u128 {} }
        impl Shl<u8> for u128 { type Output = u128; fn shl(self, _rhs: u8) -> u128 {} }
        impl Shl<&u8> for u128 { type Output = u128; fn shl(self, _rhs: &u8) -> u128 {} }
        impl Shl<u8> for &u128 { type Output = u128; fn shl(self, _rhs: u8) -> u128 {} }
        impl Shl<&u8> for &u128 { type Output = u128; fn shl(self, _rhs: &u8) -> u128 {} }
        impl Shl<u16> for u128 { type Output = u128; fn shl(self, _rhs: u16) -> u128 {} }
        impl Shl<&u16> for u128 { type Output = u128; fn shl(self, _rhs: &u16) -> u128 {} }
        impl Shl<u16> for &u128 { type Output = u128; fn shl(self, _rhs: u16) -> u128 {} }
        impl Shl<&u16> for &u128 { type Output = u128; fn shl(self, _rhs: &u16) -> u128 {} }
        impl Shl<u32> for u128 { type Output = u128; fn shl(self, _rhs: u32) -> u128 {} }
        impl Shl<&u32> for u128 { type Output = u128; fn shl(self, _rhs: &u32) -> u128 {} }
        impl Shl<u32> for &u128 { type Output = u128; fn shl(self, _rhs: u32) -> u128 {} }
        impl Shl<&u32> for &u128 { type Output = u128; fn shl(self, _rhs: &u32) -> u128 {} }
        impl Shl<u64> for u128 { type Output = u128; fn shl(self, _rhs: u64) -> u128 {} }
        impl Shl<&u64> for u128 { type Output = u128; fn shl(self, _rhs: &u64) -> u128 {} }
        impl Shl<u64> for &u128 { type Output = u128; fn shl(self, _rhs: u64) -> u128 {} }
        impl Shl<&u64> for &u128 { type Output = u128; fn shl(self, _rhs: &u64) -> u128 {} }
        impl Shl<u128> for u128 { type Output = u128; fn shl(self, _rhs: u128) -> u128 {} }
        impl Shl<&u128> for u128 { type Output = u128; fn shl(self, _rhs: &u128) -> u128 {} }
        impl Shl<u128> for &u128 { type Output = u128; fn shl(self, _rhs: u128) -> u128 {} }
        impl Shl<&u128> for &u128 { type Output = u128; fn shl(self, _rhs: &u128) -> u128 {} }
        impl Shl<usize> for u128 { type Output = u128; fn shl(self, _rhs: usize) -> u128 {} }
        impl Shl<&usize> for u128 { type Output = u128; fn shl(self, _rhs: &usize) -> u128 {} }
        impl Shl<usize> for &u128 { type Output = u128; fn shl(self, _rhs: usize) -> u128 {} }
        impl Shl<&usize> for &u128 { type Output = u128; fn shl(self, _rhs: &usize) -> u128 {} }
        impl Shl<i8> for usize { type Output = usize; fn shl(self, _rhs: i8) -> usize {} }
        impl Shl<&i8> for usize { type Output = usize; fn shl(self, _rhs: &i8) -> usize {} }
        impl Shl<i8> for &usize { type Output = usize; fn shl(self, _rhs: i8) -> usize {} }
        impl Shl<&i8> for &usize { type Output = usize; fn shl(self, _rhs: &i8) -> usize {} }
        impl Shl<i16> for usize { type Output = usize; fn shl(self, _rhs: i16) -> usize {} }
        impl Shl<&i16> for usize { type Output = usize; fn shl(self, _rhs: &i16) -> usize {} }
        impl Shl<i16> for &usize { type Output = usize; fn shl(self, _rhs: i16) -> usize {} }
        impl Shl<&i16> for &usize { type Output = usize; fn shl(self, _rhs: &i16) -> usize {} }
        impl Shl<i32> for usize { type Output = usize; fn shl(self, _rhs: i32) -> usize {} }
        impl Shl<&i32> for usize { type Output = usize; fn shl(self, _rhs: &i32) -> usize {} }
        impl Shl<i32> for &usize { type Output = usize; fn shl(self, _rhs: i32) -> usize {} }
        impl Shl<&i32> for &usize { type Output = usize; fn shl(self, _rhs: &i32) -> usize {} }
        impl Shl<i64> for usize { type Output = usize; fn shl(self, _rhs: i64) -> usize {} }
        impl Shl<&i64> for usize { type Output = usize; fn shl(self, _rhs: &i64) -> usize {} }
        impl Shl<i64> for &usize { type Output = usize; fn shl(self, _rhs: i64) -> usize {} }
        impl Shl<&i64> for &usize { type Output = usize; fn shl(self, _rhs: &i64) -> usize {} }
        impl Shl<i128> for usize { type Output = usize; fn shl(self, _rhs: i128) -> usize {} }
        impl Shl<&i128> for usize { type Output = usize; fn shl(self, _rhs: &i128) -> usize {} }
        impl Shl<i128> for &usize { type Output = usize; fn shl(self, _rhs: i128) -> usize {} }
        impl Shl<&i128> for &usize { type Output = usize; fn shl(self, _rhs: &i128) -> usize {} }
        impl Shl<isize> for usize { type Output = usize; fn shl(self, _rhs: isize) -> usize {} }
        impl Shl<&isize> for usize { type Output = usize; fn shl(self, _rhs: &isize) -> usize {} }
        impl Shl<isize> for &usize { type Output = usize; fn shl(self, _rhs: isize) -> usize {} }
        impl Shl<&isize> for &usize { type Output = usize; fn shl(self, _rhs: &isize) -> usize {} }
        impl Shl<u8> for usize { type Output = usize; fn shl(self, _rhs: u8) -> usize {} }
        impl Shl<&u8> for usize { type Output = usize; fn shl(self, _rhs: &u8) -> usize {} }
        impl Shl<u8> for &usize { type Output = usize; fn shl(self, _rhs: u8) -> usize {} }
        impl Shl<&u8> for &usize { type Output = usize; fn shl(self, _rhs: &u8) -> usize {} }
        impl Shl<u16> for usize { type Output = usize; fn shl(self, _rhs: u16) -> usize {} }
        impl Shl<&u16> for usize { type Output = usize; fn shl(self, _rhs: &u16) -> usize {} }
        impl Shl<u16> for &usize { type Output = usize; fn shl(self, _rhs: u16) -> usize {} }
        impl Shl<&u16> for &usize { type Output = usize; fn shl(self, _rhs: &u16) -> usize {} }
        impl Shl<u32> for usize { type Output = usize; fn shl(self, _rhs: u32) -> usize {} }
        impl Shl<&u32> for usize { type Output = usize; fn shl(self, _rhs: &u32) -> usize {} }
        impl Shl<u32> for &usize { type Output = usize; fn shl(self, _rhs: u32) -> usize {} }
        impl Shl<&u32> for &usize { type Output = usize; fn shl(self, _rhs: &u32) -> usize {} }
        impl Shl<u64> for usize { type Output = usize; fn shl(self, _rhs: u64) -> usize {} }
        impl Shl<&u64> for usize { type Output = usize; fn shl(self, _rhs: &u64) -> usize {} }
        impl Shl<u64> for &usize { type Output = usize; fn shl(self, _rhs: u64) -> usize {} }
        impl Shl<&u64> for &usize { type Output = usize; fn shl(self, _rhs: &u64) -> usize {} }
        impl Shl<u128> for usize { type Output = usize; fn shl(self, _rhs: u128) -> usize {} }
        impl Shl<&u128> for usize { type Output = usize; fn shl(self, _rhs: &u128) -> usize {} }
        impl Shl<u128> for &usize { type Output = usize; fn shl(self, _rhs: u128) -> usize {} }
        impl Shl<&u128> for &usize { type Output = usize; fn shl(self, _rhs: &u128) -> usize {} }
        impl Shl<usize> for usize { type Output = usize; fn shl(self, _rhs: usize) -> usize {} }
        impl Shl<&usize> for usize { type Output = usize; fn shl(self, _rhs: &usize) -> usize {} }
        impl Shl<usize> for &usize { type Output = usize; fn shl(self, _rhs: usize) -> usize {} }
        impl Shl<&usize> for &usize { type Output = usize; fn shl(self, _rhs: &usize) -> usize {} }

        pub trait Shr<Rhs = Self> { type Output; fn shr(self, rhs: Rhs) -> Self::Output; }
        impl Shr<i8> for i8 { type Output = i8; fn shr(self, _rhs: i8) -> i8 {} }
        impl Shr<&i8> for i8 { type Output = i8; fn shr(self, _rhs: &i8) -> i8 {} }
        impl Shr<i8> for &i8 { type Output = i8; fn shr(self, _rhs: i8) -> i8 {} }
        impl Shr<&i8> for &i8 { type Output = i8; fn shr(self, _rhs: &i8) -> i8 {} }
        impl Shr<i16> for i8 { type Output = i8; fn shr(self, _rhs: i16) -> i8 {} }
        impl Shr<&i16> for i8 { type Output = i8; fn shr(self, _rhs: &i16) -> i8 {} }
        impl Shr<i16> for &i8 { type Output = i8; fn shr(self, _rhs: i16) -> i8 {} }
        impl Shr<&i16> for &i8 { type Output = i8; fn shr(self, _rhs: &i16) -> i8 {} }
        impl Shr<i32> for i8 { type Output = i8; fn shr(self, _rhs: i32) -> i8 {} }
        impl Shr<&i32> for i8 { type Output = i8; fn shr(self, _rhs: &i32) -> i8 {} }
        impl Shr<i32> for &i8 { type Output = i8; fn shr(self, _rhs: i32) -> i8 {} }
        impl Shr<&i32> for &i8 { type Output = i8; fn shr(self, _rhs: &i32) -> i8 {} }
        impl Shr<i64> for i8 { type Output = i8; fn shr(self, _rhs: i64) -> i8 {} }
        impl Shr<&i64> for i8 { type Output = i8; fn shr(self, _rhs: &i64) -> i8 {} }
        impl Shr<i64> for &i8 { type Output = i8; fn shr(self, _rhs: i64) -> i8 {} }
        impl Shr<&i64> for &i8 { type Output = i8; fn shr(self, _rhs: &i64) -> i8 {} }
        impl Shr<i128> for i8 { type Output = i8; fn shr(self, _rhs: i128) -> i8 {} }
        impl Shr<&i128> for i8 { type Output = i8; fn shr(self, _rhs: &i128) -> i8 {} }
        impl Shr<i128> for &i8 { type Output = i8; fn shr(self, _rhs: i128) -> i8 {} }
        impl Shr<&i128> for &i8 { type Output = i8; fn shr(self, _rhs: &i128) -> i8 {} }
        impl Shr<isize> for i8 { type Output = i8; fn shr(self, _rhs: isize) -> i8 {} }
        impl Shr<&isize> for i8 { type Output = i8; fn shr(self, _rhs: &isize) -> i8 {} }
        impl Shr<isize> for &i8 { type Output = i8; fn shr(self, _rhs: isize) -> i8 {} }
        impl Shr<&isize> for &i8 { type Output = i8; fn shr(self, _rhs: &isize) -> i8 {} }
        impl Shr<u8> for i8 { type Output = i8; fn shr(self, _rhs: u8) -> i8 {} }
        impl Shr<&u8> for i8 { type Output = i8; fn shr(self, _rhs: &u8) -> i8 {} }
        impl Shr<u8> for &i8 { type Output = i8; fn shr(self, _rhs: u8) -> i8 {} }
        impl Shr<&u8> for &i8 { type Output = i8; fn shr(self, _rhs: &u8) -> i8 {} }
        impl Shr<u16> for i8 { type Output = i8; fn shr(self, _rhs: u16) -> i8 {} }
        impl Shr<&u16> for i8 { type Output = i8; fn shr(self, _rhs: &u16) -> i8 {} }
        impl Shr<u16> for &i8 { type Output = i8; fn shr(self, _rhs: u16) -> i8 {} }
        impl Shr<&u16> for &i8 { type Output = i8; fn shr(self, _rhs: &u16) -> i8 {} }
        impl Shr<u32> for i8 { type Output = i8; fn shr(self, _rhs: u32) -> i8 {} }
        impl Shr<&u32> for i8 { type Output = i8; fn shr(self, _rhs: &u32) -> i8 {} }
        impl Shr<u32> for &i8 { type Output = i8; fn shr(self, _rhs: u32) -> i8 {} }
        impl Shr<&u32> for &i8 { type Output = i8; fn shr(self, _rhs: &u32) -> i8 {} }
        impl Shr<u64> for i8 { type Output = i8; fn shr(self, _rhs: u64) -> i8 {} }
        impl Shr<&u64> for i8 { type Output = i8; fn shr(self, _rhs: &u64) -> i8 {} }
        impl Shr<u64> for &i8 { type Output = i8; fn shr(self, _rhs: u64) -> i8 {} }
        impl Shr<&u64> for &i8 { type Output = i8; fn shr(self, _rhs: &u64) -> i8 {} }
        impl Shr<u128> for i8 { type Output = i8; fn shr(self, _rhs: u128) -> i8 {} }
        impl Shr<&u128> for i8 { type Output = i8; fn shr(self, _rhs: &u128) -> i8 {} }
        impl Shr<u128> for &i8 { type Output = i8; fn shr(self, _rhs: u128) -> i8 {} }
        impl Shr<&u128> for &i8 { type Output = i8; fn shr(self, _rhs: &u128) -> i8 {} }
        impl Shr<usize> for i8 { type Output = i8; fn shr(self, _rhs: usize) -> i8 {} }
        impl Shr<&usize> for i8 { type Output = i8; fn shr(self, _rhs: &usize) -> i8 {} }
        impl Shr<usize> for &i8 { type Output = i8; fn shr(self, _rhs: usize) -> i8 {} }
        impl Shr<&usize> for &i8 { type Output = i8; fn shr(self, _rhs: &usize) -> i8 {} }
        impl Shr<i8> for i16 { type Output = i16; fn shr(self, _rhs: i8) -> i16 {} }
        impl Shr<&i8> for i16 { type Output = i16; fn shr(self, _rhs: &i8) -> i16 {} }
        impl Shr<i8> for &i16 { type Output = i16; fn shr(self, _rhs: i8) -> i16 {} }
        impl Shr<&i8> for &i16 { type Output = i16; fn shr(self, _rhs: &i8) -> i16 {} }
        impl Shr<i16> for i16 { type Output = i16; fn shr(self, _rhs: i16) -> i16 {} }
        impl Shr<&i16> for i16 { type Output = i16; fn shr(self, _rhs: &i16) -> i16 {} }
        impl Shr<i16> for &i16 { type Output = i16; fn shr(self, _rhs: i16) -> i16 {} }
        impl Shr<&i16> for &i16 { type Output = i16; fn shr(self, _rhs: &i16) -> i16 {} }
        impl Shr<i32> for i16 { type Output = i16; fn shr(self, _rhs: i32) -> i16 {} }
        impl Shr<&i32> for i16 { type Output = i16; fn shr(self, _rhs: &i32) -> i16 {} }
        impl Shr<i32> for &i16 { type Output = i16; fn shr(self, _rhs: i32) -> i16 {} }
        impl Shr<&i32> for &i16 { type Output = i16; fn shr(self, _rhs: &i32) -> i16 {} }
        impl Shr<i64> for i16 { type Output = i16; fn shr(self, _rhs: i64) -> i16 {} }
        impl Shr<&i64> for i16 { type Output = i16; fn shr(self, _rhs: &i64) -> i16 {} }
        impl Shr<i64> for &i16 { type Output = i16; fn shr(self, _rhs: i64) -> i16 {} }
        impl Shr<&i64> for &i16 { type Output = i16; fn shr(self, _rhs: &i64) -> i16 {} }
        impl Shr<i128> for i16 { type Output = i16; fn shr(self, _rhs: i128) -> i16 {} }
        impl Shr<&i128> for i16 { type Output = i16; fn shr(self, _rhs: &i128) -> i16 {} }
        impl Shr<i128> for &i16 { type Output = i16; fn shr(self, _rhs: i128) -> i16 {} }
        impl Shr<&i128> for &i16 { type Output = i16; fn shr(self, _rhs: &i128) -> i16 {} }
        impl Shr<isize> for i16 { type Output = i16; fn shr(self, _rhs: isize) -> i16 {} }
        impl Shr<&isize> for i16 { type Output = i16; fn shr(self, _rhs: &isize) -> i16 {} }
        impl Shr<isize> for &i16 { type Output = i16; fn shr(self, _rhs: isize) -> i16 {} }
        impl Shr<&isize> for &i16 { type Output = i16; fn shr(self, _rhs: &isize) -> i16 {} }
        impl Shr<u8> for i16 { type Output = i16; fn shr(self, _rhs: u8) -> i16 {} }
        impl Shr<&u8> for i16 { type Output = i16; fn shr(self, _rhs: &u8) -> i16 {} }
        impl Shr<u8> for &i16 { type Output = i16; fn shr(self, _rhs: u8) -> i16 {} }
        impl Shr<&u8> for &i16 { type Output = i16; fn shr(self, _rhs: &u8) -> i16 {} }
        impl Shr<u16> for i16 { type Output = i16; fn shr(self, _rhs: u16) -> i16 {} }
        impl Shr<&u16> for i16 { type Output = i16; fn shr(self, _rhs: &u16) -> i16 {} }
        impl Shr<u16> for &i16 { type Output = i16; fn shr(self, _rhs: u16) -> i16 {} }
        impl Shr<&u16> for &i16 { type Output = i16; fn shr(self, _rhs: &u16) -> i16 {} }
        impl Shr<u32> for i16 { type Output = i16; fn shr(self, _rhs: u32) -> i16 {} }
        impl Shr<&u32> for i16 { type Output = i16; fn shr(self, _rhs: &u32) -> i16 {} }
        impl Shr<u32> for &i16 { type Output = i16; fn shr(self, _rhs: u32) -> i16 {} }
        impl Shr<&u32> for &i16 { type Output = i16; fn shr(self, _rhs: &u32) -> i16 {} }
        impl Shr<u64> for i16 { type Output = i16; fn shr(self, _rhs: u64) -> i16 {} }
        impl Shr<&u64> for i16 { type Output = i16; fn shr(self, _rhs: &u64) -> i16 {} }
        impl Shr<u64> for &i16 { type Output = i16; fn shr(self, _rhs: u64) -> i16 {} }
        impl Shr<&u64> for &i16 { type Output = i16; fn shr(self, _rhs: &u64) -> i16 {} }
        impl Shr<u128> for i16 { type Output = i16; fn shr(self, _rhs: u128) -> i16 {} }
        impl Shr<&u128> for i16 { type Output = i16; fn shr(self, _rhs: &u128) -> i16 {} }
        impl Shr<u128> for &i16 { type Output = i16; fn shr(self, _rhs: u128) -> i16 {} }
        impl Shr<&u128> for &i16 { type Output = i16; fn shr(self, _rhs: &u128) -> i16 {} }
        impl Shr<usize> for i16 { type Output = i16; fn shr(self, _rhs: usize) -> i16 {} }
        impl Shr<&usize> for i16 { type Output = i16; fn shr(self, _rhs: &usize) -> i16 {} }
        impl Shr<usize> for &i16 { type Output = i16; fn shr(self, _rhs: usize) -> i16 {} }
        impl Shr<&usize> for &i16 { type Output = i16; fn shr(self, _rhs: &usize) -> i16 {} }
        impl Shr<i8> for i32 { type Output = i32; fn shr(self, _rhs: i8) -> i32 {} }
        impl Shr<&i8> for i32 { type Output = i32; fn shr(self, _rhs: &i8) -> i32 {} }
        impl Shr<i8> for &i32 { type Output = i32; fn shr(self, _rhs: i8) -> i32 {} }
        impl Shr<&i8> for &i32 { type Output = i32; fn shr(self, _rhs: &i8) -> i32 {} }
        impl Shr<i16> for i32 { type Output = i32; fn shr(self, _rhs: i16) -> i32 {} }
        impl Shr<&i16> for i32 { type Output = i32; fn shr(self, _rhs: &i16) -> i32 {} }
        impl Shr<i16> for &i32 { type Output = i32; fn shr(self, _rhs: i16) -> i32 {} }
        impl Shr<&i16> for &i32 { type Output = i32; fn shr(self, _rhs: &i16) -> i32 {} }
        impl Shr<i32> for i32 { type Output = i32; fn shr(self, _rhs: i32) -> i32 {} }
        impl Shr<&i32> for i32 { type Output = i32; fn shr(self, _rhs: &i32) -> i32 {} }
        impl Shr<i32> for &i32 { type Output = i32; fn shr(self, _rhs: i32) -> i32 {} }
        impl Shr<&i32> for &i32 { type Output = i32; fn shr(self, _rhs: &i32) -> i32 {} }
        impl Shr<i64> for i32 { type Output = i32; fn shr(self, _rhs: i64) -> i32 {} }
        impl Shr<&i64> for i32 { type Output = i32; fn shr(self, _rhs: &i64) -> i32 {} }
        impl Shr<i64> for &i32 { type Output = i32; fn shr(self, _rhs: i64) -> i32 {} }
        impl Shr<&i64> for &i32 { type Output = i32; fn shr(self, _rhs: &i64) -> i32 {} }
        impl Shr<i128> for i32 { type Output = i32; fn shr(self, _rhs: i128) -> i32 {} }
        impl Shr<&i128> for i32 { type Output = i32; fn shr(self, _rhs: &i128) -> i32 {} }
        impl Shr<i128> for &i32 { type Output = i32; fn shr(self, _rhs: i128) -> i32 {} }
        impl Shr<&i128> for &i32 { type Output = i32; fn shr(self, _rhs: &i128) -> i32 {} }
        impl Shr<isize> for i32 { type Output = i32; fn shr(self, _rhs: isize) -> i32 {} }
        impl Shr<&isize> for i32 { type Output = i32; fn shr(self, _rhs: &isize) -> i32 {} }
        impl Shr<isize> for &i32 { type Output = i32; fn shr(self, _rhs: isize) -> i32 {} }
        impl Shr<&isize> for &i32 { type Output = i32; fn shr(self, _rhs: &isize) -> i32 {} }
        impl Shr<u8> for i32 { type Output = i32; fn shr(self, _rhs: u8) -> i32 {} }
        impl Shr<&u8> for i32 { type Output = i32; fn shr(self, _rhs: &u8) -> i32 {} }
        impl Shr<u8> for &i32 { type Output = i32; fn shr(self, _rhs: u8) -> i32 {} }
        impl Shr<&u8> for &i32 { type Output = i32; fn shr(self, _rhs: &u8) -> i32 {} }
        impl Shr<u16> for i32 { type Output = i32; fn shr(self, _rhs: u16) -> i32 {} }
        impl Shr<&u16> for i32 { type Output = i32; fn shr(self, _rhs: &u16) -> i32 {} }
        impl Shr<u16> for &i32 { type Output = i32; fn shr(self, _rhs: u16) -> i32 {} }
        impl Shr<&u16> for &i32 { type Output = i32; fn shr(self, _rhs: &u16) -> i32 {} }
        impl Shr<u32> for i32 { type Output = i32; fn shr(self, _rhs: u32) -> i32 {} }
        impl Shr<&u32> for i32 { type Output = i32; fn shr(self, _rhs: &u32) -> i32 {} }
        impl Shr<u32> for &i32 { type Output = i32; fn shr(self, _rhs: u32) -> i32 {} }
        impl Shr<&u32> for &i32 { type Output = i32; fn shr(self, _rhs: &u32) -> i32 {} }
        impl Shr<u64> for i32 { type Output = i32; fn shr(self, _rhs: u64) -> i32 {} }
        impl Shr<&u64> for i32 { type Output = i32; fn shr(self, _rhs: &u64) -> i32 {} }
        impl Shr<u64> for &i32 { type Output = i32; fn shr(self, _rhs: u64) -> i32 {} }
        impl Shr<&u64> for &i32 { type Output = i32; fn shr(self, _rhs: &u64) -> i32 {} }
        impl Shr<u128> for i32 { type Output = i32; fn shr(self, _rhs: u128) -> i32 {} }
        impl Shr<&u128> for i32 { type Output = i32; fn shr(self, _rhs: &u128) -> i32 {} }
        impl Shr<u128> for &i32 { type Output = i32; fn shr(self, _rhs: u128) -> i32 {} }
        impl Shr<&u128> for &i32 { type Output = i32; fn shr(self, _rhs: &u128) -> i32 {} }
        impl Shr<usize> for i32 { type Output = i32; fn shr(self, _rhs: usize) -> i32 {} }
        impl Shr<&usize> for i32 { type Output = i32; fn shr(self, _rhs: &usize) -> i32 {} }
        impl Shr<usize> for &i32 { type Output = i32; fn shr(self, _rhs: usize) -> i32 {} }
        impl Shr<&usize> for &i32 { type Output = i32; fn shr(self, _rhs: &usize) -> i32 {} }
        impl Shr<i8> for i64 { type Output = i64; fn shr(self, _rhs: i8) -> i64 {} }
        impl Shr<&i8> for i64 { type Output = i64; fn shr(self, _rhs: &i8) -> i64 {} }
        impl Shr<i8> for &i64 { type Output = i64; fn shr(self, _rhs: i8) -> i64 {} }
        impl Shr<&i8> for &i64 { type Output = i64; fn shr(self, _rhs: &i8) -> i64 {} }
        impl Shr<i16> for i64 { type Output = i64; fn shr(self, _rhs: i16) -> i64 {} }
        impl Shr<&i16> for i64 { type Output = i64; fn shr(self, _rhs: &i16) -> i64 {} }
        impl Shr<i16> for &i64 { type Output = i64; fn shr(self, _rhs: i16) -> i64 {} }
        impl Shr<&i16> for &i64 { type Output = i64; fn shr(self, _rhs: &i16) -> i64 {} }
        impl Shr<i32> for i64 { type Output = i64; fn shr(self, _rhs: i32) -> i64 {} }
        impl Shr<&i32> for i64 { type Output = i64; fn shr(self, _rhs: &i32) -> i64 {} }
        impl Shr<i32> for &i64 { type Output = i64; fn shr(self, _rhs: i32) -> i64 {} }
        impl Shr<&i32> for &i64 { type Output = i64; fn shr(self, _rhs: &i32) -> i64 {} }
        impl Shr<i64> for i64 { type Output = i64; fn shr(self, _rhs: i64) -> i64 {} }
        impl Shr<&i64> for i64 { type Output = i64; fn shr(self, _rhs: &i64) -> i64 {} }
        impl Shr<i64> for &i64 { type Output = i64; fn shr(self, _rhs: i64) -> i64 {} }
        impl Shr<&i64> for &i64 { type Output = i64; fn shr(self, _rhs: &i64) -> i64 {} }
        impl Shr<i128> for i64 { type Output = i64; fn shr(self, _rhs: i128) -> i64 {} }
        impl Shr<&i128> for i64 { type Output = i64; fn shr(self, _rhs: &i128) -> i64 {} }
        impl Shr<i128> for &i64 { type Output = i64; fn shr(self, _rhs: i128) -> i64 {} }
        impl Shr<&i128> for &i64 { type Output = i64; fn shr(self, _rhs: &i128) -> i64 {} }
        impl Shr<isize> for i64 { type Output = i64; fn shr(self, _rhs: isize) -> i64 {} }
        impl Shr<&isize> for i64 { type Output = i64; fn shr(self, _rhs: &isize) -> i64 {} }
        impl Shr<isize> for &i64 { type Output = i64; fn shr(self, _rhs: isize) -> i64 {} }
        impl Shr<&isize> for &i64 { type Output = i64; fn shr(self, _rhs: &isize) -> i64 {} }
        impl Shr<u8> for i64 { type Output = i64; fn shr(self, _rhs: u8) -> i64 {} }
        impl Shr<&u8> for i64 { type Output = i64; fn shr(self, _rhs: &u8) -> i64 {} }
        impl Shr<u8> for &i64 { type Output = i64; fn shr(self, _rhs: u8) -> i64 {} }
        impl Shr<&u8> for &i64 { type Output = i64; fn shr(self, _rhs: &u8) -> i64 {} }
        impl Shr<u16> for i64 { type Output = i64; fn shr(self, _rhs: u16) -> i64 {} }
        impl Shr<&u16> for i64 { type Output = i64; fn shr(self, _rhs: &u16) -> i64 {} }
        impl Shr<u16> for &i64 { type Output = i64; fn shr(self, _rhs: u16) -> i64 {} }
        impl Shr<&u16> for &i64 { type Output = i64; fn shr(self, _rhs: &u16) -> i64 {} }
        impl Shr<u32> for i64 { type Output = i64; fn shr(self, _rhs: u32) -> i64 {} }
        impl Shr<&u32> for i64 { type Output = i64; fn shr(self, _rhs: &u32) -> i64 {} }
        impl Shr<u32> for &i64 { type Output = i64; fn shr(self, _rhs: u32) -> i64 {} }
        impl Shr<&u32> for &i64 { type Output = i64; fn shr(self, _rhs: &u32) -> i64 {} }
        impl Shr<u64> for i64 { type Output = i64; fn shr(self, _rhs: u64) -> i64 {} }
        impl Shr<&u64> for i64 { type Output = i64; fn shr(self, _rhs: &u64) -> i64 {} }
        impl Shr<u64> for &i64 { type Output = i64; fn shr(self, _rhs: u64) -> i64 {} }
        impl Shr<&u64> for &i64 { type Output = i64; fn shr(self, _rhs: &u64) -> i64 {} }
        impl Shr<u128> for i64 { type Output = i64; fn shr(self, _rhs: u128) -> i64 {} }
        impl Shr<&u128> for i64 { type Output = i64; fn shr(self, _rhs: &u128) -> i64 {} }
        impl Shr<u128> for &i64 { type Output = i64; fn shr(self, _rhs: u128) -> i64 {} }
        impl Shr<&u128> for &i64 { type Output = i64; fn shr(self, _rhs: &u128) -> i64 {} }
        impl Shr<usize> for i64 { type Output = i64; fn shr(self, _rhs: usize) -> i64 {} }
        impl Shr<&usize> for i64 { type Output = i64; fn shr(self, _rhs: &usize) -> i64 {} }
        impl Shr<usize> for &i64 { type Output = i64; fn shr(self, _rhs: usize) -> i64 {} }
        impl Shr<&usize> for &i64 { type Output = i64; fn shr(self, _rhs: &usize) -> i64 {} }
        impl Shr<i8> for i128 { type Output = i128; fn shr(self, _rhs: i8) -> i128 {} }
        impl Shr<&i8> for i128 { type Output = i128; fn shr(self, _rhs: &i8) -> i128 {} }
        impl Shr<i8> for &i128 { type Output = i128; fn shr(self, _rhs: i8) -> i128 {} }
        impl Shr<&i8> for &i128 { type Output = i128; fn shr(self, _rhs: &i8) -> i128 {} }
        impl Shr<i16> for i128 { type Output = i128; fn shr(self, _rhs: i16) -> i128 {} }
        impl Shr<&i16> for i128 { type Output = i128; fn shr(self, _rhs: &i16) -> i128 {} }
        impl Shr<i16> for &i128 { type Output = i128; fn shr(self, _rhs: i16) -> i128 {} }
        impl Shr<&i16> for &i128 { type Output = i128; fn shr(self, _rhs: &i16) -> i128 {} }
        impl Shr<i32> for i128 { type Output = i128; fn shr(self, _rhs: i32) -> i128 {} }
        impl Shr<&i32> for i128 { type Output = i128; fn shr(self, _rhs: &i32) -> i128 {} }
        impl Shr<i32> for &i128 { type Output = i128; fn shr(self, _rhs: i32) -> i128 {} }
        impl Shr<&i32> for &i128 { type Output = i128; fn shr(self, _rhs: &i32) -> i128 {} }
        impl Shr<i64> for i128 { type Output = i128; fn shr(self, _rhs: i64) -> i128 {} }
        impl Shr<&i64> for i128 { type Output = i128; fn shr(self, _rhs: &i64) -> i128 {} }
        impl Shr<i64> for &i128 { type Output = i128; fn shr(self, _rhs: i64) -> i128 {} }
        impl Shr<&i64> for &i128 { type Output = i128; fn shr(self, _rhs: &i64) -> i128 {} }
        impl Shr<i128> for i128 { type Output = i128; fn shr(self, _rhs: i128) -> i128 {} }
        impl Shr<&i128> for i128 { type Output = i128; fn shr(self, _rhs: &i128) -> i128 {} }
        impl Shr<i128> for &i128 { type Output = i128; fn shr(self, _rhs: i128) -> i128 {} }
        impl Shr<&i128> for &i128 { type Output = i128; fn shr(self, _rhs: &i128) -> i128 {} }
        impl Shr<isize> for i128 { type Output = i128; fn shr(self, _rhs: isize) -> i128 {} }
        impl Shr<&isize> for i128 { type Output = i128; fn shr(self, _rhs: &isize) -> i128 {} }
        impl Shr<isize> for &i128 { type Output = i128; fn shr(self, _rhs: isize) -> i128 {} }
        impl Shr<&isize> for &i128 { type Output = i128; fn shr(self, _rhs: &isize) -> i128 {} }
        impl Shr<u8> for i128 { type Output = i128; fn shr(self, _rhs: u8) -> i128 {} }
        impl Shr<&u8> for i128 { type Output = i128; fn shr(self, _rhs: &u8) -> i128 {} }
        impl Shr<u8> for &i128 { type Output = i128; fn shr(self, _rhs: u8) -> i128 {} }
        impl Shr<&u8> for &i128 { type Output = i128; fn shr(self, _rhs: &u8) -> i128 {} }
        impl Shr<u16> for i128 { type Output = i128; fn shr(self, _rhs: u16) -> i128 {} }
        impl Shr<&u16> for i128 { type Output = i128; fn shr(self, _rhs: &u16) -> i128 {} }
        impl Shr<u16> for &i128 { type Output = i128; fn shr(self, _rhs: u16) -> i128 {} }
        impl Shr<&u16> for &i128 { type Output = i128; fn shr(self, _rhs: &u16) -> i128 {} }
        impl Shr<u32> for i128 { type Output = i128; fn shr(self, _rhs: u32) -> i128 {} }
        impl Shr<&u32> for i128 { type Output = i128; fn shr(self, _rhs: &u32) -> i128 {} }
        impl Shr<u32> for &i128 { type Output = i128; fn shr(self, _rhs: u32) -> i128 {} }
        impl Shr<&u32> for &i128 { type Output = i128; fn shr(self, _rhs: &u32) -> i128 {} }
        impl Shr<u64> for i128 { type Output = i128; fn shr(self, _rhs: u64) -> i128 {} }
        impl Shr<&u64> for i128 { type Output = i128; fn shr(self, _rhs: &u64) -> i128 {} }
        impl Shr<u64> for &i128 { type Output = i128; fn shr(self, _rhs: u64) -> i128 {} }
        impl Shr<&u64> for &i128 { type Output = i128; fn shr(self, _rhs: &u64) -> i128 {} }
        impl Shr<u128> for i128 { type Output = i128; fn shr(self, _rhs: u128) -> i128 {} }
        impl Shr<&u128> for i128 { type Output = i128; fn shr(self, _rhs: &u128) -> i128 {} }
        impl Shr<u128> for &i128 { type Output = i128; fn shr(self, _rhs: u128) -> i128 {} }
        impl Shr<&u128> for &i128 { type Output = i128; fn shr(self, _rhs: &u128) -> i128 {} }
        impl Shr<usize> for i128 { type Output = i128; fn shr(self, _rhs: usize) -> i128 {} }
        impl Shr<&usize> for i128 { type Output = i128; fn shr(self, _rhs: &usize) -> i128 {} }
        impl Shr<usize> for &i128 { type Output = i128; fn shr(self, _rhs: usize) -> i128 {} }
        impl Shr<&usize> for &i128 { type Output = i128; fn shr(self, _rhs: &usize) -> i128 {} }
        impl Shr<i8> for isize { type Output = isize; fn shr(self, _rhs: i8) -> isize {} }
        impl Shr<&i8> for isize { type Output = isize; fn shr(self, _rhs: &i8) -> isize {} }
        impl Shr<i8> for &isize { type Output = isize; fn shr(self, _rhs: i8) -> isize {} }
        impl Shr<&i8> for &isize { type Output = isize; fn shr(self, _rhs: &i8) -> isize {} }
        impl Shr<i16> for isize { type Output = isize; fn shr(self, _rhs: i16) -> isize {} }
        impl Shr<&i16> for isize { type Output = isize; fn shr(self, _rhs: &i16) -> isize {} }
        impl Shr<i16> for &isize { type Output = isize; fn shr(self, _rhs: i16) -> isize {} }
        impl Shr<&i16> for &isize { type Output = isize; fn shr(self, _rhs: &i16) -> isize {} }
        impl Shr<i32> for isize { type Output = isize; fn shr(self, _rhs: i32) -> isize {} }
        impl Shr<&i32> for isize { type Output = isize; fn shr(self, _rhs: &i32) -> isize {} }
        impl Shr<i32> for &isize { type Output = isize; fn shr(self, _rhs: i32) -> isize {} }
        impl Shr<&i32> for &isize { type Output = isize; fn shr(self, _rhs: &i32) -> isize {} }
        impl Shr<i64> for isize { type Output = isize; fn shr(self, _rhs: i64) -> isize {} }
        impl Shr<&i64> for isize { type Output = isize; fn shr(self, _rhs: &i64) -> isize {} }
        impl Shr<i64> for &isize { type Output = isize; fn shr(self, _rhs: i64) -> isize {} }
        impl Shr<&i64> for &isize { type Output = isize; fn shr(self, _rhs: &i64) -> isize {} }
        impl Shr<i128> for isize { type Output = isize; fn shr(self, _rhs: i128) -> isize {} }
        impl Shr<&i128> for isize { type Output = isize; fn shr(self, _rhs: &i128) -> isize {} }
        impl Shr<i128> for &isize { type Output = isize; fn shr(self, _rhs: i128) -> isize {} }
        impl Shr<&i128> for &isize { type Output = isize; fn shr(self, _rhs: &i128) -> isize {} }
        impl Shr<isize> for isize { type Output = isize; fn shr(self, _rhs: isize) -> isize {} }
        impl Shr<&isize> for isize { type Output = isize; fn shr(self, _rhs: &isize) -> isize {} }
        impl Shr<isize> for &isize { type Output = isize; fn shr(self, _rhs: isize) -> isize {} }
        impl Shr<&isize> for &isize { type Output = isize; fn shr(self, _rhs: &isize) -> isize {} }
        impl Shr<u8> for isize { type Output = isize; fn shr(self, _rhs: u8) -> isize {} }
        impl Shr<&u8> for isize { type Output = isize; fn shr(self, _rhs: &u8) -> isize {} }
        impl Shr<u8> for &isize { type Output = isize; fn shr(self, _rhs: u8) -> isize {} }
        impl Shr<&u8> for &isize { type Output = isize; fn shr(self, _rhs: &u8) -> isize {} }
        impl Shr<u16> for isize { type Output = isize; fn shr(self, _rhs: u16) -> isize {} }
        impl Shr<&u16> for isize { type Output = isize; fn shr(self, _rhs: &u16) -> isize {} }
        impl Shr<u16> for &isize { type Output = isize; fn shr(self, _rhs: u16) -> isize {} }
        impl Shr<&u16> for &isize { type Output = isize; fn shr(self, _rhs: &u16) -> isize {} }
        impl Shr<u32> for isize { type Output = isize; fn shr(self, _rhs: u32) -> isize {} }
        impl Shr<&u32> for isize { type Output = isize; fn shr(self, _rhs: &u32) -> isize {} }
        impl Shr<u32> for &isize { type Output = isize; fn shr(self, _rhs: u32) -> isize {} }
        impl Shr<&u32> for &isize { type Output = isize; fn shr(self, _rhs: &u32) -> isize {} }
        impl Shr<u64> for isize { type Output = isize; fn shr(self, _rhs: u64) -> isize {} }
        impl Shr<&u64> for isize { type Output = isize; fn shr(self, _rhs: &u64) -> isize {} }
        impl Shr<u64> for &isize { type Output = isize; fn shr(self, _rhs: u64) -> isize {} }
        impl Shr<&u64> for &isize { type Output = isize; fn shr(self, _rhs: &u64) -> isize {} }
        impl Shr<u128> for isize { type Output = isize; fn shr(self, _rhs: u128) -> isize {} }
        impl Shr<&u128> for isize { type Output = isize; fn shr(self, _rhs: &u128) -> isize {} }
        impl Shr<u128> for &isize { type Output = isize; fn shr(self, _rhs: u128) -> isize {} }
        impl Shr<&u128> for &isize { type Output = isize; fn shr(self, _rhs: &u128) -> isize {} }
        impl Shr<usize> for isize { type Output = isize; fn shr(self, _rhs: usize) -> isize {} }
        impl Shr<&usize> for isize { type Output = isize; fn shr(self, _rhs: &usize) -> isize {} }
        impl Shr<usize> for &isize { type Output = isize; fn shr(self, _rhs: usize) -> isize {} }
        impl Shr<&usize> for &isize { type Output = isize; fn shr(self, _rhs: &usize) -> isize {} }
        impl Shr<i8> for u8 { type Output = u8; fn shr(self, _rhs: i8) -> u8 {} }
        impl Shr<&i8> for u8 { type Output = u8; fn shr(self, _rhs: &i8) -> u8 {} }
        impl Shr<i8> for &u8 { type Output = u8; fn shr(self, _rhs: i8) -> u8 {} }
        impl Shr<&i8> for &u8 { type Output = u8; fn shr(self, _rhs: &i8) -> u8 {} }
        impl Shr<i16> for u8 { type Output = u8; fn shr(self, _rhs: i16) -> u8 {} }
        impl Shr<&i16> for u8 { type Output = u8; fn shr(self, _rhs: &i16) -> u8 {} }
        impl Shr<i16> for &u8 { type Output = u8; fn shr(self, _rhs: i16) -> u8 {} }
        impl Shr<&i16> for &u8 { type Output = u8; fn shr(self, _rhs: &i16) -> u8 {} }
        impl Shr<i32> for u8 { type Output = u8; fn shr(self, _rhs: i32) -> u8 {} }
        impl Shr<&i32> for u8 { type Output = u8; fn shr(self, _rhs: &i32) -> u8 {} }
        impl Shr<i32> for &u8 { type Output = u8; fn shr(self, _rhs: i32) -> u8 {} }
        impl Shr<&i32> for &u8 { type Output = u8; fn shr(self, _rhs: &i32) -> u8 {} }
        impl Shr<i64> for u8 { type Output = u8; fn shr(self, _rhs: i64) -> u8 {} }
        impl Shr<&i64> for u8 { type Output = u8; fn shr(self, _rhs: &i64) -> u8 {} }
        impl Shr<i64> for &u8 { type Output = u8; fn shr(self, _rhs: i64) -> u8 {} }
        impl Shr<&i64> for &u8 { type Output = u8; fn shr(self, _rhs: &i64) -> u8 {} }
        impl Shr<i128> for u8 { type Output = u8; fn shr(self, _rhs: i128) -> u8 {} }
        impl Shr<&i128> for u8 { type Output = u8; fn shr(self, _rhs: &i128) -> u8 {} }
        impl Shr<i128> for &u8 { type Output = u8; fn shr(self, _rhs: i128) -> u8 {} }
        impl Shr<&i128> for &u8 { type Output = u8; fn shr(self, _rhs: &i128) -> u8 {} }
        impl Shr<isize> for u8 { type Output = u8; fn shr(self, _rhs: isize) -> u8 {} }
        impl Shr<&isize> for u8 { type Output = u8; fn shr(self, _rhs: &isize) -> u8 {} }
        impl Shr<isize> for &u8 { type Output = u8; fn shr(self, _rhs: isize) -> u8 {} }
        impl Shr<&isize> for &u8 { type Output = u8; fn shr(self, _rhs: &isize) -> u8 {} }
        impl Shr<u8> for u8 { type Output = u8; fn shr(self, _rhs: u8) -> u8 {} }
        impl Shr<&u8> for u8 { type Output = u8; fn shr(self, _rhs: &u8) -> u8 {} }
        impl Shr<u8> for &u8 { type Output = u8; fn shr(self, _rhs: u8) -> u8 {} }
        impl Shr<&u8> for &u8 { type Output = u8; fn shr(self, _rhs: &u8) -> u8 {} }
        impl Shr<u16> for u8 { type Output = u8; fn shr(self, _rhs: u16) -> u8 {} }
        impl Shr<&u16> for u8 { type Output = u8; fn shr(self, _rhs: &u16) -> u8 {} }
        impl Shr<u16> for &u8 { type Output = u8; fn shr(self, _rhs: u16) -> u8 {} }
        impl Shr<&u16> for &u8 { type Output = u8; fn shr(self, _rhs: &u16) -> u8 {} }
        impl Shr<u32> for u8 { type Output = u8; fn shr(self, _rhs: u32) -> u8 {} }
        impl Shr<&u32> for u8 { type Output = u8; fn shr(self, _rhs: &u32) -> u8 {} }
        impl Shr<u32> for &u8 { type Output = u8; fn shr(self, _rhs: u32) -> u8 {} }
        impl Shr<&u32> for &u8 { type Output = u8; fn shr(self, _rhs: &u32) -> u8 {} }
        impl Shr<u64> for u8 { type Output = u8; fn shr(self, _rhs: u64) -> u8 {} }
        impl Shr<&u64> for u8 { type Output = u8; fn shr(self, _rhs: &u64) -> u8 {} }
        impl Shr<u64> for &u8 { type Output = u8; fn shr(self, _rhs: u64) -> u8 {} }
        impl Shr<&u64> for &u8 { type Output = u8; fn shr(self, _rhs: &u64) -> u8 {} }
        impl Shr<u128> for u8 { type Output = u8; fn shr(self, _rhs: u128) -> u8 {} }
        impl Shr<&u128> for u8 { type Output = u8; fn shr(self, _rhs: &u128) -> u8 {} }
        impl Shr<u128> for &u8 { type Output = u8; fn shr(self, _rhs: u128) -> u8 {} }
        impl Shr<&u128> for &u8 { type Output = u8; fn shr(self, _rhs: &u128) -> u8 {} }
        impl Shr<usize> for u8 { type Output = u8; fn shr(self, _rhs: usize) -> u8 {} }
        impl Shr<&usize> for u8 { type Output = u8; fn shr(self, _rhs: &usize) -> u8 {} }
        impl Shr<usize> for &u8 { type Output = u8; fn shr(self, _rhs: usize) -> u8 {} }
        impl Shr<&usize> for &u8 { type Output = u8; fn shr(self, _rhs: &usize) -> u8 {} }
        impl Shr<i8> for u16 { type Output = u16; fn shr(self, _rhs: i8) -> u16 {} }
        impl Shr<&i8> for u16 { type Output = u16; fn shr(self, _rhs: &i8) -> u16 {} }
        impl Shr<i8> for &u16 { type Output = u16; fn shr(self, _rhs: i8) -> u16 {} }
        impl Shr<&i8> for &u16 { type Output = u16; fn shr(self, _rhs: &i8) -> u16 {} }
        impl Shr<i16> for u16 { type Output = u16; fn shr(self, _rhs: i16) -> u16 {} }
        impl Shr<&i16> for u16 { type Output = u16; fn shr(self, _rhs: &i16) -> u16 {} }
        impl Shr<i16> for &u16 { type Output = u16; fn shr(self, _rhs: i16) -> u16 {} }
        impl Shr<&i16> for &u16 { type Output = u16; fn shr(self, _rhs: &i16) -> u16 {} }
        impl Shr<i32> for u16 { type Output = u16; fn shr(self, _rhs: i32) -> u16 {} }
        impl Shr<&i32> for u16 { type Output = u16; fn shr(self, _rhs: &i32) -> u16 {} }
        impl Shr<i32> for &u16 { type Output = u16; fn shr(self, _rhs: i32) -> u16 {} }
        impl Shr<&i32> for &u16 { type Output = u16; fn shr(self, _rhs: &i32) -> u16 {} }
        impl Shr<i64> for u16 { type Output = u16; fn shr(self, _rhs: i64) -> u16 {} }
        impl Shr<&i64> for u16 { type Output = u16; fn shr(self, _rhs: &i64) -> u16 {} }
        impl Shr<i64> for &u16 { type Output = u16; fn shr(self, _rhs: i64) -> u16 {} }
        impl Shr<&i64> for &u16 { type Output = u16; fn shr(self, _rhs: &i64) -> u16 {} }
        impl Shr<i128> for u16 { type Output = u16; fn shr(self, _rhs: i128) -> u16 {} }
        impl Shr<&i128> for u16 { type Output = u16; fn shr(self, _rhs: &i128) -> u16 {} }
        impl Shr<i128> for &u16 { type Output = u16; fn shr(self, _rhs: i128) -> u16 {} }
        impl Shr<&i128> for &u16 { type Output = u16; fn shr(self, _rhs: &i128) -> u16 {} }
        impl Shr<isize> for u16 { type Output = u16; fn shr(self, _rhs: isize) -> u16 {} }
        impl Shr<&isize> for u16 { type Output = u16; fn shr(self, _rhs: &isize) -> u16 {} }
        impl Shr<isize> for &u16 { type Output = u16; fn shr(self, _rhs: isize) -> u16 {} }
        impl Shr<&isize> for &u16 { type Output = u16; fn shr(self, _rhs: &isize) -> u16 {} }
        impl Shr<u8> for u16 { type Output = u16; fn shr(self, _rhs: u8) -> u16 {} }
        impl Shr<&u8> for u16 { type Output = u16; fn shr(self, _rhs: &u8) -> u16 {} }
        impl Shr<u8> for &u16 { type Output = u16; fn shr(self, _rhs: u8) -> u16 {} }
        impl Shr<&u8> for &u16 { type Output = u16; fn shr(self, _rhs: &u8) -> u16 {} }
        impl Shr<u16> for u16 { type Output = u16; fn shr(self, _rhs: u16) -> u16 {} }
        impl Shr<&u16> for u16 { type Output = u16; fn shr(self, _rhs: &u16) -> u16 {} }
        impl Shr<u16> for &u16 { type Output = u16; fn shr(self, _rhs: u16) -> u16 {} }
        impl Shr<&u16> for &u16 { type Output = u16; fn shr(self, _rhs: &u16) -> u16 {} }
        impl Shr<u32> for u16 { type Output = u16; fn shr(self, _rhs: u32) -> u16 {} }
        impl Shr<&u32> for u16 { type Output = u16; fn shr(self, _rhs: &u32) -> u16 {} }
        impl Shr<u32> for &u16 { type Output = u16; fn shr(self, _rhs: u32) -> u16 {} }
        impl Shr<&u32> for &u16 { type Output = u16; fn shr(self, _rhs: &u32) -> u16 {} }
        impl Shr<u64> for u16 { type Output = u16; fn shr(self, _rhs: u64) -> u16 {} }
        impl Shr<&u64> for u16 { type Output = u16; fn shr(self, _rhs: &u64) -> u16 {} }
        impl Shr<u64> for &u16 { type Output = u16; fn shr(self, _rhs: u64) -> u16 {} }
        impl Shr<&u64> for &u16 { type Output = u16; fn shr(self, _rhs: &u64) -> u16 {} }
        impl Shr<u128> for u16 { type Output = u16; fn shr(self, _rhs: u128) -> u16 {} }
        impl Shr<&u128> for u16 { type Output = u16; fn shr(self, _rhs: &u128) -> u16 {} }
        impl Shr<u128> for &u16 { type Output = u16; fn shr(self, _rhs: u128) -> u16 {} }
        impl Shr<&u128> for &u16 { type Output = u16; fn shr(self, _rhs: &u128) -> u16 {} }
        impl Shr<usize> for u16 { type Output = u16; fn shr(self, _rhs: usize) -> u16 {} }
        impl Shr<&usize> for u16 { type Output = u16; fn shr(self, _rhs: &usize) -> u16 {} }
        impl Shr<usize> for &u16 { type Output = u16; fn shr(self, _rhs: usize) -> u16 {} }
        impl Shr<&usize> for &u16 { type Output = u16; fn shr(self, _rhs: &usize) -> u16 {} }
        impl Shr<i8> for u32 { type Output = u32; fn shr(self, _rhs: i8) -> u32 {} }
        impl Shr<&i8> for u32 { type Output = u32; fn shr(self, _rhs: &i8) -> u32 {} }
        impl Shr<i8> for &u32 { type Output = u32; fn shr(self, _rhs: i8) -> u32 {} }
        impl Shr<&i8> for &u32 { type Output = u32; fn shr(self, _rhs: &i8) -> u32 {} }
        impl Shr<i16> for u32 { type Output = u32; fn shr(self, _rhs: i16) -> u32 {} }
        impl Shr<&i16> for u32 { type Output = u32; fn shr(self, _rhs: &i16) -> u32 {} }
        impl Shr<i16> for &u32 { type Output = u32; fn shr(self, _rhs: i16) -> u32 {} }
        impl Shr<&i16> for &u32 { type Output = u32; fn shr(self, _rhs: &i16) -> u32 {} }
        impl Shr<i32> for u32 { type Output = u32; fn shr(self, _rhs: i32) -> u32 {} }
        impl Shr<&i32> for u32 { type Output = u32; fn shr(self, _rhs: &i32) -> u32 {} }
        impl Shr<i32> for &u32 { type Output = u32; fn shr(self, _rhs: i32) -> u32 {} }
        impl Shr<&i32> for &u32 { type Output = u32; fn shr(self, _rhs: &i32) -> u32 {} }
        impl Shr<i64> for u32 { type Output = u32; fn shr(self, _rhs: i64) -> u32 {} }
        impl Shr<&i64> for u32 { type Output = u32; fn shr(self, _rhs: &i64) -> u32 {} }
        impl Shr<i64> for &u32 { type Output = u32; fn shr(self, _rhs: i64) -> u32 {} }
        impl Shr<&i64> for &u32 { type Output = u32; fn shr(self, _rhs: &i64) -> u32 {} }
        impl Shr<i128> for u32 { type Output = u32; fn shr(self, _rhs: i128) -> u32 {} }
        impl Shr<&i128> for u32 { type Output = u32; fn shr(self, _rhs: &i128) -> u32 {} }
        impl Shr<i128> for &u32 { type Output = u32; fn shr(self, _rhs: i128) -> u32 {} }
        impl Shr<&i128> for &u32 { type Output = u32; fn shr(self, _rhs: &i128) -> u32 {} }
        impl Shr<isize> for u32 { type Output = u32; fn shr(self, _rhs: isize) -> u32 {} }
        impl Shr<&isize> for u32 { type Output = u32; fn shr(self, _rhs: &isize) -> u32 {} }
        impl Shr<isize> for &u32 { type Output = u32; fn shr(self, _rhs: isize) -> u32 {} }
        impl Shr<&isize> for &u32 { type Output = u32; fn shr(self, _rhs: &isize) -> u32 {} }
        impl Shr<u8> for u32 { type Output = u32; fn shr(self, _rhs: u8) -> u32 {} }
        impl Shr<&u8> for u32 { type Output = u32; fn shr(self, _rhs: &u8) -> u32 {} }
        impl Shr<u8> for &u32 { type Output = u32; fn shr(self, _rhs: u8) -> u32 {} }
        impl Shr<&u8> for &u32 { type Output = u32; fn shr(self, _rhs: &u8) -> u32 {} }
        impl Shr<u16> for u32 { type Output = u32; fn shr(self, _rhs: u16) -> u32 {} }
        impl Shr<&u16> for u32 { type Output = u32; fn shr(self, _rhs: &u16) -> u32 {} }
        impl Shr<u16> for &u32 { type Output = u32; fn shr(self, _rhs: u16) -> u32 {} }
        impl Shr<&u16> for &u32 { type Output = u32; fn shr(self, _rhs: &u16) -> u32 {} }
        impl Shr<u32> for u32 { type Output = u32; fn shr(self, _rhs: u32) -> u32 {} }
        impl Shr<&u32> for u32 { type Output = u32; fn shr(self, _rhs: &u32) -> u32 {} }
        impl Shr<u32> for &u32 { type Output = u32; fn shr(self, _rhs: u32) -> u32 {} }
        impl Shr<&u32> for &u32 { type Output = u32; fn shr(self, _rhs: &u32) -> u32 {} }
        impl Shr<u64> for u32 { type Output = u32; fn shr(self, _rhs: u64) -> u32 {} }
        impl Shr<&u64> for u32 { type Output = u32; fn shr(self, _rhs: &u64) -> u32 {} }
        impl Shr<u64> for &u32 { type Output = u32; fn shr(self, _rhs: u64) -> u32 {} }
        impl Shr<&u64> for &u32 { type Output = u32; fn shr(self, _rhs: &u64) -> u32 {} }
        impl Shr<u128> for u32 { type Output = u32; fn shr(self, _rhs: u128) -> u32 {} }
        impl Shr<&u128> for u32 { type Output = u32; fn shr(self, _rhs: &u128) -> u32 {} }
        impl Shr<u128> for &u32 { type Output = u32; fn shr(self, _rhs: u128) -> u32 {} }
        impl Shr<&u128> for &u32 { type Output = u32; fn shr(self, _rhs: &u128) -> u32 {} }
        impl Shr<usize> for u32 { type Output = u32; fn shr(self, _rhs: usize) -> u32 {} }
        impl Shr<&usize> for u32 { type Output = u32; fn shr(self, _rhs: &usize) -> u32 {} }
        impl Shr<usize> for &u32 { type Output = u32; fn shr(self, _rhs: usize) -> u32 {} }
        impl Shr<&usize> for &u32 { type Output = u32; fn shr(self, _rhs: &usize) -> u32 {} }
        impl Shr<i8> for u64 { type Output = u64; fn shr(self, _rhs: i8) -> u64 {} }
        impl Shr<&i8> for u64 { type Output = u64; fn shr(self, _rhs: &i8) -> u64 {} }
        impl Shr<i8> for &u64 { type Output = u64; fn shr(self, _rhs: i8) -> u64 {} }
        impl Shr<&i8> for &u64 { type Output = u64; fn shr(self, _rhs: &i8) -> u64 {} }
        impl Shr<i16> for u64 { type Output = u64; fn shr(self, _rhs: i16) -> u64 {} }
        impl Shr<&i16> for u64 { type Output = u64; fn shr(self, _rhs: &i16) -> u64 {} }
        impl Shr<i16> for &u64 { type Output = u64; fn shr(self, _rhs: i16) -> u64 {} }
        impl Shr<&i16> for &u64 { type Output = u64; fn shr(self, _rhs: &i16) -> u64 {} }
        impl Shr<i32> for u64 { type Output = u64; fn shr(self, _rhs: i32) -> u64 {} }
        impl Shr<&i32> for u64 { type Output = u64; fn shr(self, _rhs: &i32) -> u64 {} }
        impl Shr<i32> for &u64 { type Output = u64; fn shr(self, _rhs: i32) -> u64 {} }
        impl Shr<&i32> for &u64 { type Output = u64; fn shr(self, _rhs: &i32) -> u64 {} }
        impl Shr<i64> for u64 { type Output = u64; fn shr(self, _rhs: i64) -> u64 {} }
        impl Shr<&i64> for u64 { type Output = u64; fn shr(self, _rhs: &i64) -> u64 {} }
        impl Shr<i64> for &u64 { type Output = u64; fn shr(self, _rhs: i64) -> u64 {} }
        impl Shr<&i64> for &u64 { type Output = u64; fn shr(self, _rhs: &i64) -> u64 {} }
        impl Shr<i128> for u64 { type Output = u64; fn shr(self, _rhs: i128) -> u64 {} }
        impl Shr<&i128> for u64 { type Output = u64; fn shr(self, _rhs: &i128) -> u64 {} }
        impl Shr<i128> for &u64 { type Output = u64; fn shr(self, _rhs: i128) -> u64 {} }
        impl Shr<&i128> for &u64 { type Output = u64; fn shr(self, _rhs: &i128) -> u64 {} }
        impl Shr<isize> for u64 { type Output = u64; fn shr(self, _rhs: isize) -> u64 {} }
        impl Shr<&isize> for u64 { type Output = u64; fn shr(self, _rhs: &isize) -> u64 {} }
        impl Shr<isize> for &u64 { type Output = u64; fn shr(self, _rhs: isize) -> u64 {} }
        impl Shr<&isize> for &u64 { type Output = u64; fn shr(self, _rhs: &isize) -> u64 {} }
        impl Shr<u8> for u64 { type Output = u64; fn shr(self, _rhs: u8) -> u64 {} }
        impl Shr<&u8> for u64 { type Output = u64; fn shr(self, _rhs: &u8) -> u64 {} }
        impl Shr<u8> for &u64 { type Output = u64; fn shr(self, _rhs: u8) -> u64 {} }
        impl Shr<&u8> for &u64 { type Output = u64; fn shr(self, _rhs: &u8) -> u64 {} }
        impl Shr<u16> for u64 { type Output = u64; fn shr(self, _rhs: u16) -> u64 {} }
        impl Shr<&u16> for u64 { type Output = u64; fn shr(self, _rhs: &u16) -> u64 {} }
        impl Shr<u16> for &u64 { type Output = u64; fn shr(self, _rhs: u16) -> u64 {} }
        impl Shr<&u16> for &u64 { type Output = u64; fn shr(self, _rhs: &u16) -> u64 {} }
        impl Shr<u32> for u64 { type Output = u64; fn shr(self, _rhs: u32) -> u64 {} }
        impl Shr<&u32> for u64 { type Output = u64; fn shr(self, _rhs: &u32) -> u64 {} }
        impl Shr<u32> for &u64 { type Output = u64; fn shr(self, _rhs: u32) -> u64 {} }
        impl Shr<&u32> for &u64 { type Output = u64; fn shr(self, _rhs: &u32) -> u64 {} }
        impl Shr<u64> for u64 { type Output = u64; fn shr(self, _rhs: u64) -> u64 {} }
        impl Shr<&u64> for u64 { type Output = u64; fn shr(self, _rhs: &u64) -> u64 {} }
        impl Shr<u64> for &u64 { type Output = u64; fn shr(self, _rhs: u64) -> u64 {} }
        impl Shr<&u64> for &u64 { type Output = u64; fn shr(self, _rhs: &u64) -> u64 {} }
        impl Shr<u128> for u64 { type Output = u64; fn shr(self, _rhs: u128) -> u64 {} }
        impl Shr<&u128> for u64 { type Output = u64; fn shr(self, _rhs: &u128) -> u64 {} }
        impl Shr<u128> for &u64 { type Output = u64; fn shr(self, _rhs: u128) -> u64 {} }
        impl Shr<&u128> for &u64 { type Output = u64; fn shr(self, _rhs: &u128) -> u64 {} }
        impl Shr<usize> for u64 { type Output = u64; fn shr(self, _rhs: usize) -> u64 {} }
        impl Shr<&usize> for u64 { type Output = u64; fn shr(self, _rhs: &usize) -> u64 {} }
        impl Shr<usize> for &u64 { type Output = u64; fn shr(self, _rhs: usize) -> u64 {} }
        impl Shr<&usize> for &u64 { type Output = u64; fn shr(self, _rhs: &usize) -> u64 {} }
        impl Shr<i8> for u128 { type Output = u128; fn shr(self, _rhs: i8) -> u128 {} }
        impl Shr<&i8> for u128 { type Output = u128; fn shr(self, _rhs: &i8) -> u128 {} }
        impl Shr<i8> for &u128 { type Output = u128; fn shr(self, _rhs: i8) -> u128 {} }
        impl Shr<&i8> for &u128 { type Output = u128; fn shr(self, _rhs: &i8) -> u128 {} }
        impl Shr<i16> for u128 { type Output = u128; fn shr(self, _rhs: i16) -> u128 {} }
        impl Shr<&i16> for u128 { type Output = u128; fn shr(self, _rhs: &i16) -> u128 {} }
        impl Shr<i16> for &u128 { type Output = u128; fn shr(self, _rhs: i16) -> u128 {} }
        impl Shr<&i16> for &u128 { type Output = u128; fn shr(self, _rhs: &i16) -> u128 {} }
        impl Shr<i32> for u128 { type Output = u128; fn shr(self, _rhs: i32) -> u128 {} }
        impl Shr<&i32> for u128 { type Output = u128; fn shr(self, _rhs: &i32) -> u128 {} }
        impl Shr<i32> for &u128 { type Output = u128; fn shr(self, _rhs: i32) -> u128 {} }
        impl Shr<&i32> for &u128 { type Output = u128; fn shr(self, _rhs: &i32) -> u128 {} }
        impl Shr<i64> for u128 { type Output = u128; fn shr(self, _rhs: i64) -> u128 {} }
        impl Shr<&i64> for u128 { type Output = u128; fn shr(self, _rhs: &i64) -> u128 {} }
        impl Shr<i64> for &u128 { type Output = u128; fn shr(self, _rhs: i64) -> u128 {} }
        impl Shr<&i64> for &u128 { type Output = u128; fn shr(self, _rhs: &i64) -> u128 {} }
        impl Shr<i128> for u128 { type Output = u128; fn shr(self, _rhs: i128) -> u128 {} }
        impl Shr<&i128> for u128 { type Output = u128; fn shr(self, _rhs: &i128) -> u128 {} }
        impl Shr<i128> for &u128 { type Output = u128; fn shr(self, _rhs: i128) -> u128 {} }
        impl Shr<&i128> for &u128 { type Output = u128; fn shr(self, _rhs: &i128) -> u128 {} }
        impl Shr<isize> for u128 { type Output = u128; fn shr(self, _rhs: isize) -> u128 {} }
        impl Shr<&isize> for u128 { type Output = u128; fn shr(self, _rhs: &isize) -> u128 {} }
        impl Shr<isize> for &u128 { type Output = u128; fn shr(self, _rhs: isize) -> u128 {} }
        impl Shr<&isize> for &u128 { type Output = u128; fn shr(self, _rhs: &isize) -> u128 {} }
        impl Shr<u8> for u128 { type Output = u128; fn shr(self, _rhs: u8) -> u128 {} }
        impl Shr<&u8> for u128 { type Output = u128; fn shr(self, _rhs: &u8) -> u128 {} }
        impl Shr<u8> for &u128 { type Output = u128; fn shr(self, _rhs: u8) -> u128 {} }
        impl Shr<&u8> for &u128 { type Output = u128; fn shr(self, _rhs: &u8) -> u128 {} }
        impl Shr<u16> for u128 { type Output = u128; fn shr(self, _rhs: u16) -> u128 {} }
        impl Shr<&u16> for u128 { type Output = u128; fn shr(self, _rhs: &u16) -> u128 {} }
        impl Shr<u16> for &u128 { type Output = u128; fn shr(self, _rhs: u16) -> u128 {} }
        impl Shr<&u16> for &u128 { type Output = u128; fn shr(self, _rhs: &u16) -> u128 {} }
        impl Shr<u32> for u128 { type Output = u128; fn shr(self, _rhs: u32) -> u128 {} }
        impl Shr<&u32> for u128 { type Output = u128; fn shr(self, _rhs: &u32) -> u128 {} }
        impl Shr<u32> for &u128 { type Output = u128; fn shr(self, _rhs: u32) -> u128 {} }
        impl Shr<&u32> for &u128 { type Output = u128; fn shr(self, _rhs: &u32) -> u128 {} }
        impl Shr<u64> for u128 { type Output = u128; fn shr(self, _rhs: u64) -> u128 {} }
        impl Shr<&u64> for u128 { type Output = u128; fn shr(self, _rhs: &u64) -> u128 {} }
        impl Shr<u64> for &u128 { type Output = u128; fn shr(self, _rhs: u64) -> u128 {} }
        impl Shr<&u64> for &u128 { type Output = u128; fn shr(self, _rhs: &u64) -> u128 {} }
        impl Shr<u128> for u128 { type Output = u128; fn shr(self, _rhs: u128) -> u128 {} }
        impl Shr<&u128> for u128 { type Output = u128; fn shr(self, _rhs: &u128) -> u128 {} }
        impl Shr<u128> for &u128 { type Output = u128; fn shr(self, _rhs: u128) -> u128 {} }
        impl Shr<&u128> for &u128 { type Output = u128; fn shr(self, _rhs: &u128) -> u128 {} }
        impl Shr<usize> for u128 { type Output = u128; fn shr(self, _rhs: usize) -> u128 {} }
        impl Shr<&usize> for u128 { type Output = u128; fn shr(self, _rhs: &usize) -> u128 {} }
        impl Shr<usize> for &u128 { type Output = u128; fn shr(self, _rhs: usize) -> u128 {} }
        impl Shr<&usize> for &u128 { type Output = u128; fn shr(self, _rhs: &usize) -> u128 {} }
        impl Shr<i8> for usize { type Output = usize; fn shr(self, _rhs: i8) -> usize {} }
        impl Shr<&i8> for usize { type Output = usize; fn shr(self, _rhs: &i8) -> usize {} }
        impl Shr<i8> for &usize { type Output = usize; fn shr(self, _rhs: i8) -> usize {} }
        impl Shr<&i8> for &usize { type Output = usize; fn shr(self, _rhs: &i8) -> usize {} }
        impl Shr<i16> for usize { type Output = usize; fn shr(self, _rhs: i16) -> usize {} }
        impl Shr<&i16> for usize { type Output = usize; fn shr(self, _rhs: &i16) -> usize {} }
        impl Shr<i16> for &usize { type Output = usize; fn shr(self, _rhs: i16) -> usize {} }
        impl Shr<&i16> for &usize { type Output = usize; fn shr(self, _rhs: &i16) -> usize {} }
        impl Shr<i32> for usize { type Output = usize; fn shr(self, _rhs: i32) -> usize {} }
        impl Shr<&i32> for usize { type Output = usize; fn shr(self, _rhs: &i32) -> usize {} }
        impl Shr<i32> for &usize { type Output = usize; fn shr(self, _rhs: i32) -> usize {} }
        impl Shr<&i32> for &usize { type Output = usize; fn shr(self, _rhs: &i32) -> usize {} }
        impl Shr<i64> for usize { type Output = usize; fn shr(self, _rhs: i64) -> usize {} }
        impl Shr<&i64> for usize { type Output = usize; fn shr(self, _rhs: &i64) -> usize {} }
        impl Shr<i64> for &usize { type Output = usize; fn shr(self, _rhs: i64) -> usize {} }
        impl Shr<&i64> for &usize { type Output = usize; fn shr(self, _rhs: &i64) -> usize {} }
        impl Shr<i128> for usize { type Output = usize; fn shr(self, _rhs: i128) -> usize {} }
        impl Shr<&i128> for usize { type Output = usize; fn shr(self, _rhs: &i128) -> usize {} }
        impl Shr<i128> for &usize { type Output = usize; fn shr(self, _rhs: i128) -> usize {} }
        impl Shr<&i128> for &usize { type Output = usize; fn shr(self, _rhs: &i128) -> usize {} }
        impl Shr<isize> for usize { type Output = usize; fn shr(self, _rhs: isize) -> usize {} }
        impl Shr<&isize> for usize { type Output = usize; fn shr(self, _rhs: &isize) -> usize {} }
        impl Shr<isize> for &usize { type Output = usize; fn shr(self, _rhs: isize) -> usize {} }
        impl Shr<&isize> for &usize { type Output = usize; fn shr(self, _rhs: &isize) -> usize {} }
        impl Shr<u8> for usize { type Output = usize; fn shr(self, _rhs: u8) -> usize {} }
        impl Shr<&u8> for usize { type Output = usize; fn shr(self, _rhs: &u8) -> usize {} }
        impl Shr<u8> for &usize { type Output = usize; fn shr(self, _rhs: u8) -> usize {} }
        impl Shr<&u8> for &usize { type Output = usize; fn shr(self, _rhs: &u8) -> usize {} }
        impl Shr<u16> for usize { type Output = usize; fn shr(self, _rhs: u16) -> usize {} }
        impl Shr<&u16> for usize { type Output = usize; fn shr(self, _rhs: &u16) -> usize {} }
        impl Shr<u16> for &usize { type Output = usize; fn shr(self, _rhs: u16) -> usize {} }
        impl Shr<&u16> for &usize { type Output = usize; fn shr(self, _rhs: &u16) -> usize {} }
        impl Shr<u32> for usize { type Output = usize; fn shr(self, _rhs: u32) -> usize {} }
        impl Shr<&u32> for usize { type Output = usize; fn shr(self, _rhs: &u32) -> usize {} }
        impl Shr<u32> for &usize { type Output = usize; fn shr(self, _rhs: u32) -> usize {} }
        impl Shr<&u32> for &usize { type Output = usize; fn shr(self, _rhs: &u32) -> usize {} }
        impl Shr<u64> for usize { type Output = usize; fn shr(self, _rhs: u64) -> usize {} }
        impl Shr<&u64> for usize { type Output = usize; fn shr(self, _rhs: &u64) -> usize {} }
        impl Shr<u64> for &usize { type Output = usize; fn shr(self, _rhs: u64) -> usize {} }
        impl Shr<&u64> for &usize { type Output = usize; fn shr(self, _rhs: &u64) -> usize {} }
        impl Shr<u128> for usize { type Output = usize; fn shr(self, _rhs: u128) -> usize {} }
        impl Shr<&u128> for usize { type Output = usize; fn shr(self, _rhs: &u128) -> usize {} }
        impl Shr<u128> for &usize { type Output = usize; fn shr(self, _rhs: u128) -> usize {} }
        impl Shr<&u128> for &usize { type Output = usize; fn shr(self, _rhs: &u128) -> usize {} }
        impl Shr<usize> for usize { type Output = usize; fn shr(self, _rhs: usize) -> usize {} }
        impl Shr<&usize> for usize { type Output = usize; fn shr(self, _rhs: &usize) -> usize {} }
        impl Shr<usize> for &usize { type Output = usize; fn shr(self, _rhs: usize) -> usize {} }
        impl Shr<&usize> for &usize { type Output = usize; fn shr(self, _rhs: &usize) -> usize {} }

        // Assignment operators

        pub trait AddAssign<Rhs = Self> { fn add_assign(&mut self, rhs: Rhs); }
        impl AddAssign<i8> for i8 { fn add_assign(&mut self, _rhs: i8) {} }
        impl AddAssign<&i8> for i8 { fn add_assign(&mut self, _rhs: &i8) {} }
        impl AddAssign<i16> for i16 { fn add_assign(&mut self, _rhs: i16) {} }
        impl AddAssign<&i16> for i16 { fn add_assign(&mut self, _rhs: &i16) {} }
        impl AddAssign<i32> for i32 { fn add_assign(&mut self, _rhs: i32) {} }
        impl AddAssign<&i32> for i32 { fn add_assign(&mut self, _rhs: &i32) {} }
        impl AddAssign<i64> for i64 { fn add_assign(&mut self, _rhs: i64) {} }
        impl AddAssign<&i64> for i64 { fn add_assign(&mut self, _rhs: &i64) {} }
        impl AddAssign<i128> for i128 { fn add_assign(&mut self, _rhs: i128) {} }
        impl AddAssign<&i128> for i128 { fn add_assign(&mut self, _rhs: &i128) {} }
        impl AddAssign<isize> for isize { fn add_assign(&mut self, _rhs: isize) {} }
        impl AddAssign<&isize> for isize { fn add_assign(&mut self, _rhs: &isize) {} }
        impl AddAssign<u8> for u8 { fn add_assign(&mut self, _rhs: u8) {} }
        impl AddAssign<&u8> for u8 { fn add_assign(&mut self, _rhs: &u8) {} }
        impl AddAssign<u16> for u16 { fn add_assign(&mut self, _rhs: u16) {} }
        impl AddAssign<&u16> for u16 { fn add_assign(&mut self, _rhs: &u16) {} }
        impl AddAssign<u32> for u32 { fn add_assign(&mut self, _rhs: u32) {} }
        impl AddAssign<&u32> for u32 { fn add_assign(&mut self, _rhs: &u32) {} }
        impl AddAssign<u64> for u64 { fn add_assign(&mut self, _rhs: u64) {} }
        impl AddAssign<&u64> for u64 { fn add_assign(&mut self, _rhs: &u64) {} }
        impl AddAssign<u128> for u128 { fn add_assign(&mut self, _rhs: u128) {} }
        impl AddAssign<&u128> for u128 { fn add_assign(&mut self, _rhs: &u128) {} }
        impl AddAssign<usize> for usize { fn add_assign(&mut self, _rhs: usize) {} }
        impl AddAssign<&usize> for usize { fn add_assign(&mut self, _rhs: &usize) {} }
        impl AddAssign<f32> for f32 { fn add_assign(&mut self, _rhs: f32) {} }
        impl AddAssign<&f32> for f32 { fn add_assign(&mut self, _rhs: &f32) {} }
        impl AddAssign<f64> for f64 { fn add_assign(&mut self, _rhs: f64) {} }
        impl AddAssign<&f64> for f64 { fn add_assign(&mut self, _rhs: &f64) {} }

        pub trait SubAssign<Rhs = Self> { fn sub_assign(&mut self, rhs: Rhs); }
        impl SubAssign<i8> for i8 { fn sub_assign(&mut self, _rhs: i8) {} }
        impl SubAssign<&i8> for i8 { fn sub_assign(&mut self, _rhs: &i8) {} }
        impl SubAssign<i16> for i16 { fn sub_assign(&mut self, _rhs: i16) {} }
        impl SubAssign<&i16> for i16 { fn sub_assign(&mut self, _rhs: &i16) {} }
        impl SubAssign<i32> for i32 { fn sub_assign(&mut self, _rhs: i32) {} }
        impl SubAssign<&i32> for i32 { fn sub_assign(&mut self, _rhs: &i32) {} }
        impl SubAssign<i64> for i64 { fn sub_assign(&mut self, _rhs: i64) {} }
        impl SubAssign<&i64> for i64 { fn sub_assign(&mut self, _rhs: &i64) {} }
        impl SubAssign<i128> for i128 { fn sub_assign(&mut self, _rhs: i128) {} }
        impl SubAssign<&i128> for i128 { fn sub_assign(&mut self, _rhs: &i128) {} }
        impl SubAssign<isize> for isize { fn sub_assign(&mut self, _rhs: isize) {} }
        impl SubAssign<&isize> for isize { fn sub_assign(&mut self, _rhs: &isize) {} }
        impl SubAssign<u8> for u8 { fn sub_assign(&mut self, _rhs: u8) {} }
        impl SubAssign<&u8> for u8 { fn sub_assign(&mut self, _rhs: &u8) {} }
        impl SubAssign<u16> for u16 { fn sub_assign(&mut self, _rhs: u16) {} }
        impl SubAssign<&u16> for u16 { fn sub_assign(&mut self, _rhs: &u16) {} }
        impl SubAssign<u32> for u32 { fn sub_assign(&mut self, _rhs: u32) {} }
        impl SubAssign<&u32> for u32 { fn sub_assign(&mut self, _rhs: &u32) {} }
        impl SubAssign<u64> for u64 { fn sub_assign(&mut self, _rhs: u64) {} }
        impl SubAssign<&u64> for u64 { fn sub_assign(&mut self, _rhs: &u64) {} }
        impl SubAssign<u128> for u128 { fn sub_assign(&mut self, _rhs: u128) {} }
        impl SubAssign<&u128> for u128 { fn sub_assign(&mut self, _rhs: &u128) {} }
        impl SubAssign<usize> for usize { fn sub_assign(&mut self, _rhs: usize) {} }
        impl SubAssign<&usize> for usize { fn sub_assign(&mut self, _rhs: &usize) {} }
        impl SubAssign<f32> for f32 { fn sub_assign(&mut self, _rhs: f32) {} }
        impl SubAssign<&f32> for f32 { fn sub_assign(&mut self, _rhs: &f32) {} }
        impl SubAssign<f64> for f64 { fn sub_assign(&mut self, _rhs: f64) {} }
        impl SubAssign<&f64> for f64 { fn sub_assign(&mut self, _rhs: &f64) {} }

        pub trait MulAssign<Rhs = Self> { fn mul_assign(&mut self, rhs: Rhs); }
        impl MulAssign<i8> for i8 { fn mul_assign(&mut self, _rhs: i8) {} }
        impl MulAssign<&i8> for i8 { fn mul_assign(&mut self, _rhs: &i8) {} }
        impl MulAssign<i16> for i16 { fn mul_assign(&mut self, _rhs: i16) {} }
        impl MulAssign<&i16> for i16 { fn mul_assign(&mut self, _rhs: &i16) {} }
        impl MulAssign<i32> for i32 { fn mul_assign(&mut self, _rhs: i32) {} }
        impl MulAssign<&i32> for i32 { fn mul_assign(&mut self, _rhs: &i32) {} }
        impl MulAssign<i64> for i64 { fn mul_assign(&mut self, _rhs: i64) {} }
        impl MulAssign<&i64> for i64 { fn mul_assign(&mut self, _rhs: &i64) {} }
        impl MulAssign<i128> for i128 { fn mul_assign(&mut self, _rhs: i128) {} }
        impl MulAssign<&i128> for i128 { fn mul_assign(&mut self, _rhs: &i128) {} }
        impl MulAssign<isize> for isize { fn mul_assign(&mut self, _rhs: isize) {} }
        impl MulAssign<&isize> for isize { fn mul_assign(&mut self, _rhs: &isize) {} }
        impl MulAssign<u8> for u8 { fn mul_assign(&mut self, _rhs: u8) {} }
        impl MulAssign<&u8> for u8 { fn mul_assign(&mut self, _rhs: &u8) {} }
        impl MulAssign<u16> for u16 { fn mul_assign(&mut self, _rhs: u16) {} }
        impl MulAssign<&u16> for u16 { fn mul_assign(&mut self, _rhs: &u16) {} }
        impl MulAssign<u32> for u32 { fn mul_assign(&mut self, _rhs: u32) {} }
        impl MulAssign<&u32> for u32 { fn mul_assign(&mut self, _rhs: &u32) {} }
        impl MulAssign<u64> for u64 { fn mul_assign(&mut self, _rhs: u64) {} }
        impl MulAssign<&u64> for u64 { fn mul_assign(&mut self, _rhs: &u64) {} }
        impl MulAssign<u128> for u128 { fn mul_assign(&mut self, _rhs: u128) {} }
        impl MulAssign<&u128> for u128 { fn mul_assign(&mut self, _rhs: &u128) {} }
        impl MulAssign<usize> for usize { fn mul_assign(&mut self, _rhs: usize) {} }
        impl MulAssign<&usize> for usize { fn mul_assign(&mut self, _rhs: &usize) {} }
        impl MulAssign<f32> for f32 { fn mul_assign(&mut self, _rhs: f32) {} }
        impl MulAssign<&f32> for f32 { fn mul_assign(&mut self, _rhs: &f32) {} }
        impl MulAssign<f64> for f64 { fn mul_assign(&mut self, _rhs: f64) {} }
        impl MulAssign<&f64> for f64 { fn mul_assign(&mut self, _rhs: &f64) {} }

        pub trait DivAssign<Rhs = Self> { fn div_assign(&mut self, rhs: Rhs); }
        impl DivAssign<i8> for i8 { fn div_assign(&mut self, _rhs: i8) {} }
        impl DivAssign<&i8> for i8 { fn div_assign(&mut self, _rhs: &i8) {} }
        impl DivAssign<i16> for i16 { fn div_assign(&mut self, _rhs: i16) {} }
        impl DivAssign<&i16> for i16 { fn div_assign(&mut self, _rhs: &i16) {} }
        impl DivAssign<i32> for i32 { fn div_assign(&mut self, _rhs: i32) {} }
        impl DivAssign<&i32> for i32 { fn div_assign(&mut self, _rhs: &i32) {} }
        impl DivAssign<i64> for i64 { fn div_assign(&mut self, _rhs: i64) {} }
        impl DivAssign<&i64> for i64 { fn div_assign(&mut self, _rhs: &i64) {} }
        impl DivAssign<i128> for i128 { fn div_assign(&mut self, _rhs: i128) {} }
        impl DivAssign<&i128> for i128 { fn div_assign(&mut self, _rhs: &i128) {} }
        impl DivAssign<isize> for isize { fn div_assign(&mut self, _rhs: isize) {} }
        impl DivAssign<&isize> for isize { fn div_assign(&mut self, _rhs: &isize) {} }
        impl DivAssign<u8> for u8 { fn div_assign(&mut self, _rhs: u8) {} }
        impl DivAssign<&u8> for u8 { fn div_assign(&mut self, _rhs: &u8) {} }
        impl DivAssign<u16> for u16 { fn div_assign(&mut self, _rhs: u16) {} }
        impl DivAssign<&u16> for u16 { fn div_assign(&mut self, _rhs: &u16) {} }
        impl DivAssign<u32> for u32 { fn div_assign(&mut self, _rhs: u32) {} }
        impl DivAssign<&u32> for u32 { fn div_assign(&mut self, _rhs: &u32) {} }
        impl DivAssign<u64> for u64 { fn div_assign(&mut self, _rhs: u64) {} }
        impl DivAssign<&u64> for u64 { fn div_assign(&mut self, _rhs: &u64) {} }
        impl DivAssign<u128> for u128 { fn div_assign(&mut self, _rhs: u128) {} }
        impl DivAssign<&u128> for u128 { fn div_assign(&mut self, _rhs: &u128) {} }
        impl DivAssign<usize> for usize { fn div_assign(&mut self, _rhs: usize) {} }
        impl DivAssign<&usize> for usize { fn div_assign(&mut self, _rhs: &usize) {} }
        impl DivAssign<f32> for f32 { fn div_assign(&mut self, _rhs: f32) {} }
        impl DivAssign<&f32> for f32 { fn div_assign(&mut self, _rhs: &f32) {} }
        impl DivAssign<f64> for f64 { fn div_assign(&mut self, _rhs: f64) {} }
        impl DivAssign<&f64> for f64 { fn div_assign(&mut self, _rhs: &f64) {} }

        pub trait RemAssign<Rhs = Self> { fn rem_assign(&mut self, rhs: Rhs); }
        impl RemAssign<i8> for i8 { fn rem_assign(&mut self, _rhs: i8) {} }
        impl RemAssign<&i8> for i8 { fn rem_assign(&mut self, _rhs: &i8) {} }
        impl RemAssign<i16> for i16 { fn rem_assign(&mut self, _rhs: i16) {} }
        impl RemAssign<&i16> for i16 { fn rem_assign(&mut self, _rhs: &i16) {} }
        impl RemAssign<i32> for i32 { fn rem_assign(&mut self, _rhs: i32) {} }
        impl RemAssign<&i32> for i32 { fn rem_assign(&mut self, _rhs: &i32) {} }
        impl RemAssign<i64> for i64 { fn rem_assign(&mut self, _rhs: i64) {} }
        impl RemAssign<&i64> for i64 { fn rem_assign(&mut self, _rhs: &i64) {} }
        impl RemAssign<i128> for i128 { fn rem_assign(&mut self, _rhs: i128) {} }
        impl RemAssign<&i128> for i128 { fn rem_assign(&mut self, _rhs: &i128) {} }
        impl RemAssign<isize> for isize { fn rem_assign(&mut self, _rhs: isize) {} }
        impl RemAssign<&isize> for isize { fn rem_assign(&mut self, _rhs: &isize) {} }
        impl RemAssign<u8> for u8 { fn rem_assign(&mut self, _rhs: u8) {} }
        impl RemAssign<&u8> for u8 { fn rem_assign(&mut self, _rhs: &u8) {} }
        impl RemAssign<u16> for u16 { fn rem_assign(&mut self, _rhs: u16) {} }
        impl RemAssign<&u16> for u16 { fn rem_assign(&mut self, _rhs: &u16) {} }
        impl RemAssign<u32> for u32 { fn rem_assign(&mut self, _rhs: u32) {} }
        impl RemAssign<&u32> for u32 { fn rem_assign(&mut self, _rhs: &u32) {} }
        impl RemAssign<u64> for u64 { fn rem_assign(&mut self, _rhs: u64) {} }
        impl RemAssign<&u64> for u64 { fn rem_assign(&mut self, _rhs: &u64) {} }
        impl RemAssign<u128> for u128 { fn rem_assign(&mut self, _rhs: u128) {} }
        impl RemAssign<&u128> for u128 { fn rem_assign(&mut self, _rhs: &u128) {} }
        impl RemAssign<usize> for usize { fn rem_assign(&mut self, _rhs: usize) {} }
        impl RemAssign<&usize> for usize { fn rem_assign(&mut self, _rhs: &usize) {} }
        impl RemAssign<f32> for f32 { fn rem_assign(&mut self, _rhs: f32) {} }
        impl RemAssign<&f32> for f32 { fn rem_assign(&mut self, _rhs: &f32) {} }
        impl RemAssign<f64> for f64 { fn rem_assign(&mut self, _rhs: f64) {} }
        impl RemAssign<&f64> for f64 { fn rem_assign(&mut self, _rhs: &f64) {} }

        pub trait BitXorAssign<Rhs = Self> { fn bitxor_assign(&mut self, rhs: Rhs); }
        impl BitXorAssign<i8> for i8 { fn bitxor_assign(&mut self, _rhs: i8) {} }
        impl BitXorAssign<&i8> for i8 { fn bitxor_assign(&mut self, _rhs: &i8) {} }
        impl BitXorAssign<i16> for i16 { fn bitxor_assign(&mut self, _rhs: i16) {} }
        impl BitXorAssign<&i16> for i16 { fn bitxor_assign(&mut self, _rhs: &i16) {} }
        impl BitXorAssign<i32> for i32 { fn bitxor_assign(&mut self, _rhs: i32) {} }
        impl BitXorAssign<&i32> for i32 { fn bitxor_assign(&mut self, _rhs: &i32) {} }
        impl BitXorAssign<i64> for i64 { fn bitxor_assign(&mut self, _rhs: i64) {} }
        impl BitXorAssign<&i64> for i64 { fn bitxor_assign(&mut self, _rhs: &i64) {} }
        impl BitXorAssign<i128> for i128 { fn bitxor_assign(&mut self, _rhs: i128) {} }
        impl BitXorAssign<&i128> for i128 { fn bitxor_assign(&mut self, _rhs: &i128) {} }
        impl BitXorAssign<isize> for isize { fn bitxor_assign(&mut self, _rhs: isize) {} }
        impl BitXorAssign<&isize> for isize { fn bitxor_assign(&mut self, _rhs: &isize) {} }
        impl BitXorAssign<u8> for u8 { fn bitxor_assign(&mut self, _rhs: u8) {} }
        impl BitXorAssign<&u8> for u8 { fn bitxor_assign(&mut self, _rhs: &u8) {} }
        impl BitXorAssign<u16> for u16 { fn bitxor_assign(&mut self, _rhs: u16) {} }
        impl BitXorAssign<&u16> for u16 { fn bitxor_assign(&mut self, _rhs: &u16) {} }
        impl BitXorAssign<u32> for u32 { fn bitxor_assign(&mut self, _rhs: u32) {} }
        impl BitXorAssign<&u32> for u32 { fn bitxor_assign(&mut self, _rhs: &u32) {} }
        impl BitXorAssign<u64> for u64 { fn bitxor_assign(&mut self, _rhs: u64) {} }
        impl BitXorAssign<&u64> for u64 { fn bitxor_assign(&mut self, _rhs: &u64) {} }
        impl BitXorAssign<u128> for u128 { fn bitxor_assign(&mut self, _rhs: u128) {} }
        impl BitXorAssign<&u128> for u128 { fn bitxor_assign(&mut self, _rhs: &u128) {} }
        impl BitXorAssign<usize> for usize { fn bitxor_assign(&mut self, _rhs: usize) {} }
        impl BitXorAssign<&usize> for usize { fn bitxor_assign(&mut self, _rhs: &usize) {} }
        impl BitXorAssign<bool> for bool { fn bitxor_assign(&mut self, _rhs: bool) {} }
        impl BitXorAssign<&bool> for bool { fn bitxor_assign(&mut self, _rhs: &bool) {} }

        pub trait BitAndAssign<Rhs = Self> { fn bitand_assign(&mut self, rhs: Rhs); }
        impl BitAndAssign<i8> for i8 { fn bitand_assign(&mut self, _rhs: i8) {} }
        impl BitAndAssign<&i8> for i8 { fn bitand_assign(&mut self, _rhs: &i8) {} }
        impl BitAndAssign<i16> for i16 { fn bitand_assign(&mut self, _rhs: i16) {} }
        impl BitAndAssign<&i16> for i16 { fn bitand_assign(&mut self, _rhs: &i16) {} }
        impl BitAndAssign<i32> for i32 { fn bitand_assign(&mut self, _rhs: i32) {} }
        impl BitAndAssign<&i32> for i32 { fn bitand_assign(&mut self, _rhs: &i32) {} }
        impl BitAndAssign<i64> for i64 { fn bitand_assign(&mut self, _rhs: i64) {} }
        impl BitAndAssign<&i64> for i64 { fn bitand_assign(&mut self, _rhs: &i64) {} }
        impl BitAndAssign<i128> for i128 { fn bitand_assign(&mut self, _rhs: i128) {} }
        impl BitAndAssign<&i128> for i128 { fn bitand_assign(&mut self, _rhs: &i128) {} }
        impl BitAndAssign<isize> for isize { fn bitand_assign(&mut self, _rhs: isize) {} }
        impl BitAndAssign<&isize> for isize { fn bitand_assign(&mut self, _rhs: &isize) {} }
        impl BitAndAssign<u8> for u8 { fn bitand_assign(&mut self, _rhs: u8) {} }
        impl BitAndAssign<&u8> for u8 { fn bitand_assign(&mut self, _rhs: &u8) {} }
        impl BitAndAssign<u16> for u16 { fn bitand_assign(&mut self, _rhs: u16) {} }
        impl BitAndAssign<&u16> for u16 { fn bitand_assign(&mut self, _rhs: &u16) {} }
        impl BitAndAssign<u32> for u32 { fn bitand_assign(&mut self, _rhs: u32) {} }
        impl BitAndAssign<&u32> for u32 { fn bitand_assign(&mut self, _rhs: &u32) {} }
        impl BitAndAssign<u64> for u64 { fn bitand_assign(&mut self, _rhs: u64) {} }
        impl BitAndAssign<&u64> for u64 { fn bitand_assign(&mut self, _rhs: &u64) {} }
        impl BitAndAssign<u128> for u128 { fn bitand_assign(&mut self, _rhs: u128) {} }
        impl BitAndAssign<&u128> for u128 { fn bitand_assign(&mut self, _rhs: &u128) {} }
        impl BitAndAssign<usize> for usize { fn bitand_assign(&mut self, _rhs: usize) {} }
        impl BitAndAssign<&usize> for usize { fn bitand_assign(&mut self, _rhs: &usize) {} }
        impl BitAndAssign<bool> for bool { fn bitand_assign(&mut self, _rhs: bool) {} }
        impl BitAndAssign<&bool> for bool { fn bitand_assign(&mut self, _rhs: &bool) {} }

        pub trait BitOrAssign<Rhs = Self> { fn bitor_assign(&mut self, rhs: Rhs); }
        impl BitOrAssign<i8> for i8 { fn bitor_assign(&mut self, _rhs: i8) {} }
        impl BitOrAssign<&i8> for i8 { fn bitor_assign(&mut self, _rhs: &i8) {} }
        impl BitOrAssign<i16> for i16 { fn bitor_assign(&mut self, _rhs: i16) {} }
        impl BitOrAssign<&i16> for i16 { fn bitor_assign(&mut self, _rhs: &i16) {} }
        impl BitOrAssign<i32> for i32 { fn bitor_assign(&mut self, _rhs: i32) {} }
        impl BitOrAssign<&i32> for i32 { fn bitor_assign(&mut self, _rhs: &i32) {} }
        impl BitOrAssign<i64> for i64 { fn bitor_assign(&mut self, _rhs: i64) {} }
        impl BitOrAssign<&i64> for i64 { fn bitor_assign(&mut self, _rhs: &i64) {} }
        impl BitOrAssign<i128> for i128 { fn bitor_assign(&mut self, _rhs: i128) {} }
        impl BitOrAssign<&i128> for i128 { fn bitor_assign(&mut self, _rhs: &i128) {} }
        impl BitOrAssign<isize> for isize { fn bitor_assign(&mut self, _rhs: isize) {} }
        impl BitOrAssign<&isize> for isize { fn bitor_assign(&mut self, _rhs: &isize) {} }
        impl BitOrAssign<u8> for u8 { fn bitor_assign(&mut self, _rhs: u8) {} }
        impl BitOrAssign<&u8> for u8 { fn bitor_assign(&mut self, _rhs: &u8) {} }
        impl BitOrAssign<u16> for u16 { fn bitor_assign(&mut self, _rhs: u16) {} }
        impl BitOrAssign<&u16> for u16 { fn bitor_assign(&mut self, _rhs: &u16) {} }
        impl BitOrAssign<u32> for u32 { fn bitor_assign(&mut self, _rhs: u32) {} }
        impl BitOrAssign<&u32> for u32 { fn bitor_assign(&mut self, _rhs: &u32) {} }
        impl BitOrAssign<u64> for u64 { fn bitor_assign(&mut self, _rhs: u64) {} }
        impl BitOrAssign<&u64> for u64 { fn bitor_assign(&mut self, _rhs: &u64) {} }
        impl BitOrAssign<u128> for u128 { fn bitor_assign(&mut self, _rhs: u128) {} }
        impl BitOrAssign<&u128> for u128 { fn bitor_assign(&mut self, _rhs: &u128) {} }
        impl BitOrAssign<usize> for usize { fn bitor_assign(&mut self, _rhs: usize) {} }
        impl BitOrAssign<&usize> for usize { fn bitor_assign(&mut self, _rhs: &usize) {} }
        impl BitOrAssign<bool> for bool { fn bitor_assign(&mut self, _rhs: bool) {} }
        impl BitOrAssign<&bool> for bool { fn bitor_assign(&mut self, _rhs: &bool) {} }

        // Unary operators

        pub trait Not { type Output; fn not(self) -> Self::Output; }
        impl Not for i8 { type Output = i8; fn not(self) -> i8 {} }
        impl Not for &i8 { type Output = i8; fn not(self) -> i8 {} }
        impl Not for i16 { type Output = i16; fn not(self) -> i16 {} }
        impl Not for &i16 { type Output = i16; fn not(self) -> i16 {} }
        impl Not for i32 { type Output = i32; fn not(self) -> i32 {} }
        impl Not for &i32 { type Output = i32; fn not(self) -> i32 {} }
        impl Not for i64 { type Output = i64; fn not(self) -> i64 {} }
        impl Not for &i64 { type Output = i64; fn not(self) -> i64 {} }
        impl Not for i128 { type Output = i128; fn not(self) -> i128 {} }
        impl Not for &i128 { type Output = i128; fn not(self) -> i128 {} }
        impl Not for isize { type Output = isize; fn not(self) -> isize {} }
        impl Not for &isize { type Output = isize; fn not(self) -> isize {} }
        impl Not for u8 { type Output = u8; fn not(self) -> u8 {} }
        impl Not for &u8 { type Output = u8; fn not(self) -> u8 {} }
        impl Not for u16 { type Output = u16; fn not(self) -> u16 {} }
        impl Not for &u16 { type Output = u16; fn not(self) -> u16 {} }
        impl Not for u32 { type Output = u32; fn not(self) -> u32 {} }
        impl Not for &u32 { type Output = u32; fn not(self) -> u32 {} }
        impl Not for u64 { type Output = u64; fn not(self) -> u64 {} }
        impl Not for &u64 { type Output = u64; fn not(self) -> u64 {} }
        impl Not for u128 { type Output = u128; fn not(self) -> u128 {} }
        impl Not for &u128 { type Output = u128; fn not(self) -> u128 {} }
        impl Not for usize { type Output = usize; fn not(self) -> usize {} }
        impl Not for &usize { type Output = usize; fn not(self) -> usize {} }
        impl Not for bool { type Output = bool; fn not(self) -> bool {} }
        impl Not for &bool { type Output = bool; fn not(self) -> bool {} }

        pub trait Neg { type Output; fn neg(self) -> Self::Output; }
        impl Neg for i8 { type Output = i8; fn neg(self) -> i8 {} }
        impl Neg for &i8 { type Output = i8; fn neg(self) -> i8 {} }
        impl Neg for i16 { type Output = i16; fn neg(self) -> i16 {} }
        impl Neg for &i16 { type Output = i16; fn neg(self) -> i16 {} }
        impl Neg for i32 { type Output = i32; fn neg(self) -> i32 {} }
        impl Neg for &i32 { type Output = i32; fn neg(self) -> i32 {} }
        impl Neg for i64 { type Output = i64; fn neg(self) -> i64 {} }
        impl Neg for &i64 { type Output = i64; fn neg(self) -> i64 {} }
        impl Neg for i128 { type Output = i128; fn neg(self) -> i128 {} }
        impl Neg for &i128 { type Output = i128; fn neg(self) -> i128 {} }
        impl Neg for isize { type Output = isize; fn neg(self) -> isize {} }
        impl Neg for &isize { type Output = isize; fn neg(self) -> isize {} }
        impl Neg for f32 { type Output = f32; fn neg(self) -> f32 {} }
        impl Neg for &f32 { type Output = f32; fn neg(self) -> f32 {} }
        impl Neg for f64 { type Output = f64; fn neg(self) -> f64 {} }
        impl Neg for &f64 { type Output = f64; fn neg(self) -> f64 {} }
    }
}
