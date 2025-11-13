//! Benchmark comparing naive vs optimized DCT implementations
//!
//! Run with: cargo bench --bench dct_comparison

use criterion::{black_box, criterion_group, criterion_main, Criterion, Throughput};
use jxl_transform::{dct8x8_forward, dct8x8_inverse, dct8x8_forward_optimized, dct8x8_inverse_optimized};
use jxl_transform::{dct_channel, idct_channel, dct_channel_optimized, idct_channel_optimized};

fn bench_dct_8x8_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("DCT 8x8 Comparison");
    let input: [f32; 64] = core::array::from_fn(|i| (i as f32) / 64.0);

    group.bench_function("naive_forward", |b| {
        let mut output = [0.0f32; 64];
        b.iter(|| {
            dct8x8_forward(black_box(&input), black_box(&mut output));
        });
    });

    group.bench_function("optimized_forward", |b| {
        let mut output = [0.0f32; 64];
        b.iter(|| {
            dct8x8_forward_optimized(black_box(&input), black_box(&mut output));
        });
    });

    group.bench_function("naive_inverse", |b| {
        let mut output = [0.0f32; 64];
        b.iter(|| {
            dct8x8_inverse(black_box(&input), black_box(&mut output));
        });
    });

    group.bench_function("optimized_inverse", |b| {
        let mut output = [0.0f32; 64];
        b.iter(|| {
            dct8x8_inverse_optimized(black_box(&input), black_box(&mut output));
        });
    });

    group.finish();
}

fn bench_dct_channel_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("DCT Channel Comparison");

    for &size in &[64, 128, 256] {
        let width = size;
        let height = size;
        let pixel_count = width * height;
        let channel: Vec<f32> = (0..pixel_count).map(|i| (i % 256) as f32).collect();
        let mut output = vec![0.0f32; pixel_count];

        group.throughput(Throughput::Elements(pixel_count as u64));

        group.bench_function(format!("naive_{}x{}", width, height), |b| {
            b.iter(|| {
                dct_channel(black_box(&channel), black_box(width), black_box(height), black_box(&mut output));
            });
        });

        group.bench_function(format!("optimized_{}x{}", width, height), |b| {
            b.iter(|| {
                dct_channel_optimized(black_box(&channel), black_box(width), black_box(height), black_box(&mut output));
            });
        });
    }

    group.finish();
}

fn bench_idct_channel_comparison(c: &mut Criterion) {
    let mut group = c.benchmark_group("IDCT Channel Comparison");

    for &size in &[64, 128, 256] {
        let width = size;
        let height = size;
        let pixel_count = width * height;
        let channel: Vec<f32> = (0..pixel_count).map(|i| (i % 256) as f32).collect();
        let mut output = vec![0.0f32; pixel_count];

        group.throughput(Throughput::Elements(pixel_count as u64));

        group.bench_function(format!("naive_{}x{}", width, height), |b| {
            b.iter(|| {
                idct_channel(black_box(&channel), black_box(width), black_box(height), black_box(&mut output));
            });
        });

        group.bench_function(format!("optimized_{}x{}", width, height), |b| {
            b.iter(|| {
                idct_channel_optimized(black_box(&channel), black_box(width), black_box(height), black_box(&mut output));
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_dct_8x8_comparison,
    bench_dct_channel_comparison,
    bench_idct_channel_comparison
);
criterion_main!(benches);
