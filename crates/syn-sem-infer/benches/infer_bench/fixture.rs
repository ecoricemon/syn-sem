use criterion::black_box;
use syn_sem_ast as ast;
use syn_sem_ast::SyntaxCx;
use syn_sem_common::{known::KnownLibraries, CommonCx, FilePath};
use syn_sem_hir::HirBuilder;
use syn_sem_infer::{InferConstFacts, InferDb};
use syn_sem_name::collect::NameCollector;

pub(crate) const NO_KNOWN_LIBRARIES: KnownLibraries = KnownLibraries {
    core: false,
    std: false,
};

pub(crate) const CORE_KNOWN_LIBRARY: KnownLibraries = KnownLibraries {
    core: true,
    std: false,
};

pub(crate) fn run_analysis(source_text: &str, known: KnownLibraries) {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let entry_path = ccx.intern("infer_bench.rs");
    let entry = parse_source(&ccx, &scx, entry_path, source_text);
    let mut inputs = vec![entry];
    let mut roots = vec![entry_path];

    for known in known.sources() {
        let file_path = ccx
            .insert_virtual_file(known.path, known.source)
            .expect("known source should be stored");
        let source_text = ccx
            .source_text(file_path)
            .expect("known source text should be stored");
        scx.parse_virtual_file(file_path, source_text)
            .expect("known source should parse");
        roots.push(file_path);
        inputs.push(parse_stored_source(&scx, file_path));
    }

    let names =
        NameCollector::collect(inputs.clone(), roots).expect("name collection should succeed");
    let hir = HirBuilder::new(&names).build_files(inputs);
    let infer = InferDb::analyze(&ccx, &hir, &names, &InferConstFacts::default());
    black_box(infer);
}

fn parse_source<'cx>(
    ccx: &'cx CommonCx,
    scx: &'cx SyntaxCx<'cx>,
    file_path: FilePath<'cx>,
    source_text: &str,
) -> ast::SourceInput<'cx> {
    let source_text = ccx.intern(source_text);
    scx.parse_virtual_file(file_path, source_text)
        .expect("bench input should parse");
    parse_stored_source(scx, file_path)
}

fn parse_stored_source<'cx>(
    scx: &'cx SyntaxCx<'cx>,
    file_path: FilePath<'cx>,
) -> ast::SourceInput<'cx> {
    let file = scx
        .lookup_source(file_path)
        .expect("source should be parsed")
        .ast();
    ast::SourceInput { file_path, file }
}
