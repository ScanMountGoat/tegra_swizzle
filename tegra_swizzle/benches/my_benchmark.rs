use criterion::{black_box, criterion_group, criterion_main, Criterion};

use tegra_swizzle::{block_height_mip0, div_round_up};

pub fn div_round_up_benchmark(c: &mut Criterion) {
    c.bench_function("div_round_up", |b| b.iter(|| div_round_up(black_box(10), 4)));
}


pub fn block_height_mip0_benchmark(c: &mut Criterion) {
    c.bench_function("block_height_mip0", |b| b.iter(|| block_height_mip0(black_box(512))));
}

criterion_group!(benches, div_round_up_benchmark, block_height_mip0_benchmark);
criterion_main!(benches);
