use criterion::black_box;
use criterion::{criterion_group, criterion_main, Criterion};
use tegra_swizzle::surface::deswizzle_surface;
use tegra_swizzle::surface::BlockDim;
use tegra_swizzle::swizzle::swizzled_mip_size;
use tegra_swizzle::BlockHeight;

use criterion::BenchmarkId;
use criterion::Throughput;

fn deswizzle_surface_benchmark(c: &mut Criterion) {
    // We'll allocated the size needed by the largest run.
    // This avoids including the allocation time in the benchmark.
    let source = vec![0u8; swizzled_mip_size(512, 512, 1, BlockHeight::Sixteen, 16) * 6 * 6];

    let mut group = c.benchmark_group("deswizzle_surface");
    for size in [32, 256, 512] {
        group.throughput(Throughput::Bytes((size * size * 6) as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                deswizzle_surface(
                    size,
                    size,
                    1,
                    &source,
                    BlockDim::block_4x4(),
                    None,
                    black_box(16),
                    black_box(6),
                    black_box(6),
                )
            });
        });
    }
    group.finish();
}

criterion_group!(benches, deswizzle_surface_benchmark);
criterion_main!(benches);
