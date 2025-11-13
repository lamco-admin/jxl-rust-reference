//! End-to-end encoding/decoding benchmarks
//!
//! Run with: cargo bench --bench end_to_end

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId, Throughput};
use jxl_core::*;
use jxl_encoder::{JxlEncoder, EncoderOptions};
use jxl_decoder::JxlDecoder;

fn create_test_image(width: usize, height: usize) -> Image {
    let mut image = Image::new(
        Dimensions::new(width, height),
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    ).unwrap();

    // Create gradient pattern
    if let ImageBuffer::U8(ref mut buffer) = image.buffer {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                buffer[idx] = ((x * 255) / width) as u8;      // R
                buffer[idx + 1] = ((y * 255) / height) as u8; // G
                buffer[idx + 2] = 128;                        // B
            }
        }
    }

    image
}

fn bench_encode_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encode by Image Size");

    for &size in &[64, 128, 256, 512] {
        let image = create_test_image(size, size);
        let pixel_count = size * size;

        group.throughput(Throughput::Elements(pixel_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            &size,
            |b, _| {
                let options = EncoderOptions::default();
                let encoder = JxlEncoder::new(options);
                b.iter(|| {
                    let mut encoded = Vec::new();
                    encoder.encode(black_box(&image), black_box(&mut encoded)).unwrap();
                    encoded
                });
            },
        );
    }

    group.finish();
}

fn bench_decode_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("Decode by Image Size");

    for &size in &[64, 128, 256, 512] {
        let image = create_test_image(size, size);
        let pixel_count = size * size;

        // Pre-encode the image
        let options = EncoderOptions::default();
        let encoder = JxlEncoder::new(options);
        let mut encoded = Vec::new();
        encoder.encode(&image, &mut encoded).unwrap();

        group.throughput(Throughput::Elements(pixel_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            &size,
            |b, _| {
                let mut decoder = JxlDecoder::new();
                b.iter(|| {
                    decoder.decode(black_box(&encoded[..])).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_roundtrip_by_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("Roundtrip by Image Size");

    for &size in &[64, 128, 256] {
        let image = create_test_image(size, size);
        let pixel_count = size * size;

        group.throughput(Throughput::Elements(pixel_count as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("{}x{}", size, size)),
            &size,
            |b, _| {
                let options = EncoderOptions::default();
                let encoder = JxlEncoder::new(options);
                let mut decoder = JxlDecoder::new();

                b.iter(|| {
                    let mut encoded = Vec::new();
                    encoder.encode(black_box(&image), &mut encoded).unwrap();
                    decoder.decode(black_box(&encoded[..])).unwrap()
                });
            },
        );
    }

    group.finish();
}

fn bench_encode_by_quality(c: &mut Criterion) {
    let mut group = c.benchmark_group("Encode by Quality");

    let image = create_test_image(128, 128);

    for &quality in &[50.0, 70.0, 90.0, 95.0] {
        group.bench_with_input(
            BenchmarkId::from_parameter(format!("quality_{}", quality as u32)),
            &quality,
            |b, &q| {
                let options = EncoderOptions::default().quality(q);
                let encoder = JxlEncoder::new(options);
                b.iter(|| {
                    let mut encoded = Vec::new();
                    encoder.encode(black_box(&image), black_box(&mut encoded)).unwrap();
                    encoded
                });
            },
        );
    }

    group.finish();
}

fn bench_parallel_vs_serial(c: &mut Criterion) {
    let mut group = c.benchmark_group("Parallel Processing");

    let image = create_test_image(256, 256);

    group.bench_function("encode_256x256_with_rayon", |b| {
        let options = EncoderOptions::default();
        let encoder = JxlEncoder::new(options);
        b.iter(|| {
            let mut encoded = Vec::new();
            encoder.encode(black_box(&image), black_box(&mut encoded)).unwrap();
            encoded
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_encode_by_size,
    bench_decode_by_size,
    bench_roundtrip_by_size,
    bench_encode_by_quality,
    bench_parallel_vs_serial
);
criterion_main!(benches);
