//! Inspect inferred local variable types.
//!
//! This example analyzes a function body, finds local variables in the path tree, and turns each
//! local's internal `TypeId` into a printable owned type.

use syn_sem::{pitem, AnalysisSession, GetOwned};

fn main() -> syn_sem::Result<()> {
    // The source is small and self-contained, so use a virtual file instead of creating a temporary
    // Rust project on disk.
    let analyzed = AnalysisSession::default().run(|mut analyzer| {
        let path = "demo.rs";
        let text = r#"
            fn use_values() {
                fn takes_u32(value: u32) -> i32 { 1 }

                // `input` is an integer.
                let input = 0;

                // `output` is i32 because of the return type of `take_u32`.
                // Also, `input` is u32 because of the parameter type of `take_u32`.
                let output = takes_u32(input);

                // Explicit annotations are also represented in the semantic type tree and can be
                // read back after analysis.
                let pair: (i32, i64) = (1, 2);

                // `input` is u32, so `numbers` is [u32; 2].
                let numbers = [input, input * 2];
            }
            "#;
        analyzer.add_virtual_file(path, text);
        analyzer.analyze(path)
    })?;

    let ptree = &analyzed.sem.ptree;
    let crate_name = ptree.crate_name();

    // Function bodies are represented as block nodes. The first block inside `use_values` is named
    // `{0}` in the path tree.
    let block = format!("{crate_name}::demo::use_values::{{0}}");

    // Local variables are path tree items too. `type_id()` gives the internal semantic type, and
    // `GetOwned::get_owned` converts it into a value that can be printed, compared, or stored
    // independently.
    let input_ty = ptree.get_owned(
        pitem!(ptree, "{block}::input")
            .as_local()
            .unwrap()
            .type_id(),
    );
    let output_ty = ptree.get_owned(
        pitem!(ptree, "{block}::output")
            .as_local()
            .unwrap()
            .type_id(),
    );
    let pair_ty = ptree.get_owned(pitem!(ptree, "{block}::pair").as_local().unwrap().type_id());
    let numbers_ty = ptree.get_owned(
        pitem!(ptree, "{block}::numbers")
            .as_local()
            .unwrap()
            .type_id(),
    );

    assert_eq!(input_ty.to_string(), "u32");
    assert_eq!(output_ty.to_string(), "i32");
    assert_eq!(pair_ty.to_string(), "(i32, i64)");
    assert_eq!(numbers_ty.to_string(), "[u32; 2]");

    println!("input  -> {input_ty}");
    println!("output -> {output_ty}");
    println!("pair   -> {pair_ty}");
    println!("array  -> {numbers_ty}");

    Ok(())
}
