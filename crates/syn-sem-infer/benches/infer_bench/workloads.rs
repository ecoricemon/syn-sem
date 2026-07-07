pub(crate) const DIRECT_USER_DEFINED_PROJECTION: &str = r#"
struct A;
struct B;

trait Combine<Rhs> {
    type Output;
}

impl Combine<B> for A {
    type Output = usize;
}

struct Holder {
    field: <A as Combine<B>>::Output,
}
"#;

pub(crate) const GENERIC_IMPL_SELF_PROJECTION: &str = r#"
struct Vec<T>;

trait Iterator {
    type Item;
}

impl<T> Iterator for Vec<T> {
    type Item = T;
}

struct Output {
    field: <Vec<u32> as Iterator>::Item,
}
"#;

pub(crate) const CORE_OPS_REFERENCE_ARITHMETIC: &str = r#"
fn f(value: usize, left: &usize, right: &usize) {
    let add_both_ref = left + right;
    let add_left_value = value + right;
    let add_right_value = left + value;
    let sub_both_ref = left - right;
    let sub_left_value = value - right;
    let sub_right_value = left - value;
    let mul_both_ref = left * right;
    let mul_left_value = value * right;
    let mul_right_value = left * value;
    let div_both_ref = left / right;
    let div_left_value = value / right;
    let div_right_value = left / value;
    let rem_both_ref = left % right;
    let rem_left_value = value % right;
    let rem_right_value = left % value;
}
"#;

pub(crate) const FUNCTION_CALL_TYPE_RELATIONS: &str = r#"
fn id(x: usize) -> usize {
    x
}

fn f(a: usize) {
    let b = id(a);
    let c = id(b);
    let d = id(c);
    let e = id(d);
}
"#;
