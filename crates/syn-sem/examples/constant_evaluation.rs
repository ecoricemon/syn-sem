//! Evaluate compile-time constants and inspect array lengths.
//!
//! This example shows two semantic results: `sem.evaluated` stores evaluated constant values, and
//! `sem.ptree` stores the resolved type for a constant item.

use syn_sem::{
    pid, pitem,
    value::{Scalar, Value},
    AnalysisSession, GetOwned,
};

fn main() -> syn_sem::Result<()> {
    // This example uses `const fn`, so it keeps the default configuration that
    // loads known libraries. That lets the analyzer resolve standard primitive
    // behavior needed during constant evaluation.
    let analyzed = AnalysisSession::default().run(|mut analyzer| {
        let path = "demo.rs";
        let text = r#"
            // `A` demonstrates expression evaluation through a const function.
            const A: i32 = double(2) + 1;

            // `B` demonstrates using an evaluated expression as an array length.
            const B: [i32; double_usize(1) + 2] = [0, 0, 0];

            const fn double(value: i32) -> i32 {
                value * 2
            }

            const fn double_usize(value: usize) -> usize {
                value * 2
            }
            "#;
        analyzer.add_virtual_file(path, text);
        analyzer.analyze(path)
    })?;

    let ptree = &analyzed.sem.ptree;
    let evaluated = &analyzed.sem.evaluated;
    let crate_name = ptree.crate_name();

    // `pid!` gets the path id for an item. The evaluated-value map uses that id to find the
    // compile-time value associated with `const A`.
    let a_value = evaluated
        .get_mapped_value_by_path_id(pid!(ptree, "{crate_name}::demo::A"))
        .unwrap();

    // double(2) + 1 = 5
    assert!(matches!(a_value, Value::Scalar(Scalar::I32(5))));

    // The const item's type also records that the array length was resolved to a fixed value.
    // `get_owned` makes the type printable as `[i32; 4]`.
    let b_ty = ptree.get_owned(
        pitem!(ptree, "{crate_name}::demo::B")
            .as_const()
            .unwrap()
            .type_id(),
    );
    assert_eq!(b_ty.to_string(), "[i32; 4]");

    println!("A evaluates to {a_value:?}");
    println!("B has type {b_ty}");

    Ok(())
}
