use criterion::{criterion_group, criterion_main, Criterion};
use tegra_swizzle::swizzle::swizzle_block_linear;
use tegra_swizzle::swizzled_mip_size;
use tegra_swizzle::BlockHeight;

use criterion::BenchmarkId;
use criterion::Throughput;

fn swizzle_block_linear_benchmark(c: &mut Criterion) {
    let block_height = BlockHeight::Sixteen;
    let bytes_per_pixel = 4;
    // We'll allocated the size needed by the largest run.
    // This avoids including the allocation time in the benchmark.
    let source = vec![0u8; swizzled_mip_size(512, 512, 1, block_height, bytes_per_pixel)];

    let mut group = c.benchmark_group("swizzle_block_linear");
    for size in [0, 32, 64, 128, 256, 320, 340, 384, 448, 464, 500, 512] {
        group.throughput(Throughput::Bytes((size * size * bytes_per_pixel) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| swizzle_block_linear(size, size, 1, &source, block_height, bytes_per_pixel));
        });
    }
    group.finish();
}

criterion_group!(benches, swizzle_block_linear_benchmark);
criterion_main!(benches);
