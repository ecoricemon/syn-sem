use super::fixture::{run_analysis, NO_KNOWN_LIBRARIES};
use super::workloads::FUNCTION_CALL_TYPE_RELATIONS;
use criterion::Criterion;

pub(crate) fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("infer/type_relation");
    group.bench_function("function_call_equalities", |b| {
        b.iter(|| run_analysis(FUNCTION_CALL_TYPE_RELATIONS, NO_KNOWN_LIBRARIES));
    });
    group.finish();
}
