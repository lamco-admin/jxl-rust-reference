use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use jxl_transform::{dct8x8_forward, dct8x8_inverse, simd};

/// Benchmark DCT performance with different SIMD levels
fn benchmark_dct_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct_8x8");

    // Set up test data
    let input = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
    ];
    let mut output = [0.0f32; 64];

    // Benchmark scalar implementation
    group.bench_function("scalar", |b| {
        b.iter(|| {
            dct8x8_forward(black_box(&input), black_box(&mut output));
        });
    });

    // Benchmark SIMD implementation (automatically selects best available)
    group.bench_function("simd_auto", |b| {
        b.iter(|| {
            simd::dct_8x8_simd(black_box(&input), black_box(&mut output));
        });
    });

    group.finish();
}

/// Benchmark IDCT performance
fn benchmark_idct_simd(c: &mut Criterion) {
    let mut group = c.benchmark_group("idct_8x8");

    let input = [
        10.0, 2.0, 1.0, 0.5, 0.2, 0.1, 0.05, 0.02,
        2.0, 1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01,
        1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.0,
        0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.0, 0.0,
        0.2, 0.1, 0.05, 0.02, 0.01, 0.0, 0.0, 0.0,
        0.1, 0.05, 0.02, 0.01, 0.0, 0.0, 0.0, 0.0,
        0.05, 0.02, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0,
        0.02, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    let mut output = [0.0f32; 64];

    group.bench_function("scalar", |b| {
        b.iter(|| {
            dct8x8_inverse(black_box(&input), black_box(&mut output));
        });
    });

    group.bench_function("simd_auto", |b| {
        b.iter(|| {
            simd::idct_8x8_simd(black_box(&input), black_box(&mut output));
        });
    });

    group.finish();
}

/// Benchmark DCT throughput on different image sizes
fn benchmark_dct_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct_throughput");

    // Test different numbers of blocks
    for num_blocks in [10, 100, 1000].iter() {
        let total_samples = num_blocks * 64;
        group.throughput(Throughput::Elements(total_samples as u64));

        let input = [1.0f32; 64];
        let mut output = [0.0f32; 64];

        group.bench_with_input(
            BenchmarkId::new("scalar", num_blocks),
            num_blocks,
            |b, &num_blocks| {
                b.iter(|| {
                    for _ in 0..num_blocks {
                        dct8x8_forward(black_box(&input), black_box(&mut output));
                    }
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("simd", num_blocks),
            num_blocks,
            |b, &num_blocks| {
                b.iter(|| {
                    for _ in 0..num_blocks {
                        simd::dct_8x8_simd(black_box(&input), black_box(&mut output));
                    }
                });
            },
        );
    }

    group.finish();
}

/// Benchmark full DCT/IDCT roundtrip
fn benchmark_dct_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("dct_roundtrip");

    let input = [
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
        8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
    ];
    let mut transformed = [0.0f32; 64];
    let mut output = [0.0f32; 64];

    group.bench_function("scalar_roundtrip", |b| {
        b.iter(|| {
            dct8x8_forward(black_box(&input), black_box(&mut transformed));
            dct8x8_inverse(black_box(&transformed), black_box(&mut output));
        });
    });

    group.bench_function("simd_roundtrip", |b| {
        b.iter(|| {
            simd::dct_8x8_simd(black_box(&input), black_box(&mut transformed));
            simd::idct_8x8_simd(black_box(&transformed), black_box(&mut output));
        });
    });

    group.finish();
}

/// Benchmark RGB to XYB color conversion
fn benchmark_color_conversion(c: &mut Criterion) {
    let mut group = c.benchmark_group("color_conversion");

    // Test with different pixel counts
    for pixel_count in [64, 256, 1024, 4096].iter() {
        group.throughput(Throughput::Elements(*pixel_count as u64));

        let rgb = vec![0.5f32; pixel_count * 3];
        let mut xyb = vec![0.0f32; pixel_count * 3];

        group.bench_with_input(
            BenchmarkId::new("rgb_to_xyb_simd", pixel_count),
            pixel_count,
            |b, &pixel_count| {
                b.iter(|| {
                    simd::rgb_to_xyb_simd(
                        black_box(&rgb),
                        black_box(&mut xyb),
                        black_box(pixel_count),
                    );
                });
            },
        );
    }

    group.finish();
}

/// Print detected SIMD level
fn print_simd_info(c: &mut Criterion) {
    let level = simd::SimdLevel::detect();
    println!("\n=== SIMD Capabilities ===");
    println!("Detected SIMD level: {}", level.name());
    println!("=========================\n");

    // Dummy benchmark to satisfy criterion
    c.bench_function("simd_detection", |b| {
        b.iter(|| simd::SimdLevel::detect());
    });
}

criterion_group!(
    benches,
    print_simd_info,
    benchmark_dct_simd,
    benchmark_idct_simd,
    benchmark_dct_throughput,
    benchmark_dct_roundtrip,
    benchmark_color_conversion
);
criterion_main!(benches);
