mod a1;

use syn_sem::{self, pitem, AnalysisSession};

use std::env;

#[test]
fn test_read_file() {
    env_logger::init();

    test_read_physical_file();
    syn_locator::clear();
    test_read_virtual_file()
}

/// The physical files used here also are referenced by `test_read_virtual_file` for consistency.
fn test_read_physical_file() {
    let analyzed = AnalysisSession::default()
        .run(|anaylzer| anaylzer.analyze("tests/file/a1.rs"))
        .unwrap();
    let ptree = &analyzed.sem.ptree;
    let crate_ = ptree.crate_name();

    // crate_name/tests/file/.. -> crate_name::tests::file::..
    let test = |key: &str, expected_fpath: &str| {
        let item = pitem!(ptree, "{crate_}::tests::file::{key}");
        let fpath = item.as_mod().unwrap().file_path();
        assert_eq!(
            fpath.to_str().unwrap(),
            format!(
                "{}/tests/file/{}",
                env::current_dir().unwrap().to_str().unwrap(),
                expected_fpath
            )
        );
    };

    // From the "a1.rs"
    test("a1", "a1.rs");
    test("a1::b1", "a1/b1.rs");
    test("a1::c1", "c1.rs");
    test("a1::dx", "d1");
    test("a1::dx::d2", "d1/d2.rs");
    test("a1::e1", "a1/e1");
    test("a1::e1::e2", "a1/e1/e2.rs");
    test("a1::e1::e3", "a1/e1/e4.rs");

    // From the "a1/b1.rs"
    test("a1::b1::b2", "a1/b1/b2.rs");
}

fn test_read_virtual_file() {
    let path_a1 = "a1.rs";
    let code_a1 = include_str!("./a1.rs");

    let path_b1 = "a1/b1.rs";
    let code_b1 = include_str!("./a1/b1.rs");

    let path_b2 = "a1/b1/b2.rs";
    let code_b2 = include_str!("./a1/b1/b2.rs");

    let path_c1 = "c1.rs";
    let code_c1 = include_str!("./c1.rs");

    let path_d2 = "d1/d2.rs";
    let code_d2 = include_str!("./d1/d2.rs");

    let path_e2 = "a1/e1/e2.rs";
    let code_e2 = include_str!("./a1/e1/e2.rs");

    let path_e4 = "a1/e1/e4.rs";
    let code_e4 = include_str!("./a1/e1/e4.rs");

    let analyzed = AnalysisSession::default()
        .run(|mut analyzer| {
            analyzer.add_virtual_file(path_a1, code_a1);
            analyzer.add_virtual_file(path_b1, code_b1);
            analyzer.add_virtual_file(path_b2, code_b2);
            analyzer.add_virtual_file(path_c1, code_c1);
            analyzer.add_virtual_file(path_d2, code_d2);
            analyzer.add_virtual_file(path_e2, code_e2);
            analyzer.add_virtual_file(path_e4, code_e4);
            analyzer.analyze("a1.rs")
        })
        .unwrap();
    let ptree = &analyzed.sem.ptree;
    let crate_ = ptree.crate_name();

    let item = pitem!(ptree, "{crate_}::a1");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::b1");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1/b1.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::c1");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "c1.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::dx");
    assert_eq!(item.as_mod().unwrap().file_path().to_str().unwrap(), "d1");

    let item = pitem!(ptree, "{crate_}::a1::dx::d2");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "d1/d2.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::e1");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1/e1"
    );

    let item = pitem!(ptree, "{crate_}::a1::e1::e2");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1/e1/e2.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::e1::e3");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1/e1/e4.rs"
    );

    let item = pitem!(ptree, "{crate_}::a1::b1::b2");
    assert_eq!(
        item.as_mod().unwrap().file_path().to_str().unwrap(),
        "a1/b1/b2.rs"
    );
}
