use std::hint::black_box;
use std::iter::repeat_with;
use criterion::{criterion_group, criterion_main, Criterion};

use dev_utils::get_example_scenario;
use raptor::network::PathfindingCost;
use raptor::mc_raptor_query;

fn mc_raptor_benchmark(c: &mut Criterion) {
    let (network, start, start_time, end) = get_example_scenario();
    fastrand::seed(7);
    let costs: Vec<_> = repeat_with(|| fastrand::f32() as PathfindingCost).take(network.stop_times.len()).collect();
    c.bench_function("McRaptor", |b| b.iter(|| mc_raptor_query(&network, black_box(start), black_box(start_time), black_box(end), &costs)));
}

criterion_group!(benches, mc_raptor_benchmark);
criterion_main!(benches);
