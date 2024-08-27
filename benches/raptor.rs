use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use dev_utils::get_example_scenario;
use raptor::{csa_query, raptor_query};

fn raptor_benchmark(c: &mut Criterion) {
    let (network, start, start_time, end) = get_example_scenario();
    c.bench_function("Raptor", |b| b.iter(|| raptor_query(&network, black_box(start), black_box(start_time), black_box(end))));
}

fn csa_benchmark(c: &mut Criterion) {
    let (mut network, start, start_time, end) = get_example_scenario();
    network.build_connections();
    c.bench_function("CSA", |b| b.iter(|| csa_query(&network, black_box(start), black_box(start_time), black_box(end))));
}

criterion_group!(benches, raptor_benchmark, csa_benchmark);
criterion_main!(benches);
