mod infer_bench;

use criterion::{criterion_group, criterion_main};

criterion_group!(
    benches,
    infer_bench::projection::benches,
    infer_bench::operator::benches,
    infer_bench::type_relation::benches,
);
criterion_main!(benches);
