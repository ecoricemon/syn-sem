use criterion::black_box;
use syn_sem_ast as ast;
use syn_sem_ast::{SourceKind, SyntaxCx};
use syn_sem_common::{known::KnownLibraryConfig, CommonCx, FilePath};
use syn_sem_hir::Hir;
use syn_sem_infer::{InferConstFacts, InferDb};
use syn_sem_name::NameDb;

pub(crate) const NO_KNOWN_LIBRARIES: KnownLibraryConfig = KnownLibraryConfig {
    core: false,
    std: false,
};

pub(crate) const CORE_KNOWN_LIBRARY: KnownLibraryConfig = KnownLibraryConfig {
    core: true,
    std: false,
};

pub(crate) fn run_analysis(source_text: &str, known: KnownLibraryConfig) {
    let ccx = CommonCx::default();
    let scx = SyntaxCx::new(&ccx);
    let entry_path = ccx.intern("infer_bench.rs");
    let entry = parse_source(&ccx, &scx, entry_path, source_text);
    let mut inputs = vec![entry];
    let mut roots = vec![entry_path];

    for known in known.sources() {
        let file_path = ccx
            .insert_virtual_file(known.path, known.source_text)
            .expect("known source should be stored");
        let source_text = ccx
            .source_text(file_path)
            .expect("known source text should be stored");
        scx.parse_file(file_path, source_text, SourceKind::Known)
            .expect("known source should parse");
        roots.push(file_path);
        inputs.push(parse_stored_source(&scx, file_path));
    }

    let names = NameDb::build(inputs.clone(), roots).expect("name collection should succeed");
    let hir = Hir::build(&names, inputs);
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
    scx.parse_file(file_path, source_text, SourceKind::Virtual)
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
