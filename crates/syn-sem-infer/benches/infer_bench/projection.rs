use super::fixture::{run_analysis, NO_KNOWN_LIBRARIES};
use super::workloads::{DIRECT_USER_DEFINED_PROJECTION, GENERIC_IMPL_SELF_PROJECTION};
use criterion::Criterion;

pub(crate) fn benches(c: &mut Criterion) {
    let mut group = c.benchmark_group("infer/projection");
    group.bench_function("direct_user_defined", |b| {
        b.iter(|| run_analysis(DIRECT_USER_DEFINED_PROJECTION, NO_KNOWN_LIBRARIES));
    });
    group.bench_function("generic_impl_self", |b| {
        b.iter(|| run_analysis(GENERIC_IMPL_SELF_PROJECTION, NO_KNOWN_LIBRARIES));
    });
    group.finish();
}
