//! Analyze two in-memory Rust files and look up items in the resulting path tree.
//!
//! This is the smallest useful `syn-sem` flow: register virtual files, choose an entry file, run
//! semantic analysis, then inspect items by fully-qualified path.

use syn_sem::{pitem, AnalysisSession};

fn main() -> syn_sem::Result<()> {
    // `AnalysisSession` owns the global analysis context. The closure receives an `Analyzer`, which
    // is where source files are registered before analysis.
    let analyzed = AnalysisSession::default().run(|mut analyzer| {
        // Virtual files behave like real files for module resolution. The `pub mod model;`
        // declaration below resolves to the registered `model.rs` virtual file.
        let path = "lib.rs";
        let text = r#"
            pub mod model;

            pub fn load_user() -> model::User {
                model::User { id: 1 }
            }
            "#;
        analyzer.add_virtual_file(path, text);

        // This file is analyzed because `lib.rs` declares `mod model`.
        let path = "model.rs";
        let text = r#"
            pub struct User {
                pub id: u32,
            }
            "#;
        analyzer.add_virtual_file(path, text);

        // Start analysis at the crate root. The entry file name determines the top-level module
        // layout used by the path tree.
        analyzer.analyze("lib.rs")
    })?;

    // `ptree` is the semantic path tree: it stores modules and items by their resolved paths, not
    // just by syntax.
    let ptree = &analyzed.sem.ptree;
    let crate_name = ptree.crate_name();

    // `pitem!` is a convenience macro for examples/tests. It panics if the path is missing, which
    // keeps the example focused on successful lookup.
    let user = pitem!(ptree, "{crate_name}::model::User");
    let load_user = pitem!(ptree, "{crate_name}::load_user");

    // Items are enum-like values. Use `as_struct`, `as_fn`, and similar helpers to check which kind
    // of item was found.
    if user.as_struct().is_some() {
        println!("found struct: {crate_name}::model::User");
    }
    if load_user.as_fn().is_some() {
        println!("found function: {crate_name}::load_user");
    }

    Ok(())
}
