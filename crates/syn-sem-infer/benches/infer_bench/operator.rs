use super::fixture::{run_analysis, CORE_KNOWN_LIBRARY};
use super::workloads::CORE_OPS_REFERENCE_ARITHMETIC;
use criterion::Criterion;

pub(crate) fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("infer/operator");
    group.bench_function("core_ops_reference_arithmetic", |b| {
        b.iter(|| run_analysis(CORE_OPS_REFERENCE_ARITHMETIC, CORE_KNOWN_LIBRARY));
    });
    group.finish();
}
