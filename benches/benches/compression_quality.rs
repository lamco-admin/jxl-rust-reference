use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use jxl_bitstream::{AnsDistribution, Context, ContextModel, FrequencyBand};
use jxl_core::{ColorChannels, ColorEncoding, Dimensions, Image, ImageBuffer, PixelType};
use jxl_decoder::JxlDecoder;
use jxl_encoder::{EncoderOptions, JxlEncoder};
use std::io::Cursor;

/// Helper to create a test image with gradient pattern
fn create_test_image(width: u32, height: u32) -> Image {
    let mut image = Image::new(
        Dimensions::new(width, height),
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    if let ImageBuffer::U8(ref mut buffer) = image.buffer {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                buffer[idx] = ((x * 255) / width) as u8;
                buffer[idx + 1] = ((y * 255) / height) as u8;
                buffer[idx + 2] = 128;
            }
        }
    }

    image
}

/// Calculate PSNR between two images
fn calculate_psnr(original: &Image, decoded: &Image) -> f64 {
    let orig_buf = match &original.buffer {
        ImageBuffer::U8(buf) => buf,
        _ => panic!("Expected U8 buffer"),
    };

    let dec_buf = match &decoded.buffer {
        ImageBuffer::U8(buf) => buf,
        _ => panic!("Expected U8 buffer"),
    };

    let mut mse = 0.0;
    for (o, d) in orig_buf.iter().zip(dec_buf.iter()) {
        let diff = (*o as f64 - *d as f64);
        mse += diff * diff;
    }

    mse /= orig_buf.len() as f64;

    if mse == 0.0 {
        f64::INFINITY
    } else {
        10.0 * (255.0 * 255.0 / mse).log10()
    }
}

/// Benchmark encoding speed at different quality levels
fn benchmark_encode_quality_levels(c: &mut Criterion) {
    let mut group = c.benchmark_group("encode_quality");
    let image = create_test_image(128, 128);

    for quality in [50.0, 75.0, 90.0, 95.0].iter() {
        let pixels = (image.width() * image.height()) as u64;
        group.throughput(Throughput::Elements(pixels));

        group.bench_with_input(
            BenchmarkId::new("quality", quality),
            quality,
            |b, &quality| {
                let encoder = JxlEncoder::new(EncoderOptions::default().quality(quality));
                b.iter(|| {
                    let mut encoded = Vec::new();
                    encoder
                        .encode(black_box(&image), Cursor::new(&mut encoded))
                        .unwrap();
                    black_box(encoded);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark decoding speed
fn benchmark_decode_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("decode_speed");

    let image = create_test_image(128, 128);
    let encoder = JxlEncoder::new(EncoderOptions::default().quality(90.0));
    let mut encoded = Vec::new();
    encoder.encode(&image, Cursor::new(&mut encoded)).unwrap();

    let pixels = (image.width() * image.height()) as u64;
    group.throughput(Throughput::Elements(pixels));

    group.bench_function("decode_128x128", |b| {
        b.iter(|| {
            let mut decoder = JxlDecoder::new();
            let decoded = decoder.decode(Cursor::new(black_box(&encoded))).unwrap();
            black_box(decoded);
        });
    });

    group.finish();
}

/// Benchmark full encode/decode roundtrip
fn benchmark_roundtrip(c: &mut Criterion) {
    let mut group = c.benchmark_group("roundtrip");

    for size in [(64, 64), (128, 128), (256, 256)].iter() {
        let (width, height) = *size;
        let image = create_test_image(width, height);
        let pixels = (width * height) as u64;
        group.throughput(Throughput::Elements(pixels));

        group.bench_with_input(
            BenchmarkId::new("encode_decode", format!("{}x{}", width, height)),
            &image,
            |b, image| {
                let encoder = JxlEncoder::new(EncoderOptions::default().quality(90.0));
                b.iter(|| {
                    let mut encoded = Vec::new();
                    encoder
                        .encode(black_box(image), Cursor::new(&mut encoded))
                        .unwrap();

                    let mut decoder = JxlDecoder::new();
                    let decoded = decoder.decode(Cursor::new(&encoded)).unwrap();
                    black_box(decoded);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark compression ratio vs quality
fn benchmark_compression_ratio(c: &mut Criterion) {
    // This is more of a measurement than a benchmark
    // We'll measure once and report the results
    let image = create_test_image(256, 256);
    let original_size = match &image.buffer {
        ImageBuffer::U8(buf) => buf.len(),
        _ => 0,
    };

    println!("\n=== Compression Ratio Analysis ===");
    println!("Original size: {} bytes", original_size);

    for quality in [50.0, 75.0, 90.0, 95.0, 100.0].iter() {
        let encoder = JxlEncoder::new(EncoderOptions::default().quality(*quality));
        let mut encoded = Vec::new();
        encoder.encode(&image, Cursor::new(&mut encoded)).unwrap();

        let mut decoder = JxlDecoder::new();
        let decoded = decoder.decode(Cursor::new(&encoded)).unwrap();

        let psnr = calculate_psnr(&image, &decoded);
        let ratio = original_size as f64 / encoded.len() as f64;
        let bpp = (encoded.len() * 8) as f64 / (image.width() * image.height()) as f64;

        println!("Quality {:.0}: {} bytes, {:.2}x compression, {:.3} bpp, {:.2} dB PSNR",
                 quality, encoded.len(), ratio, bpp, psnr);
    }
    println!("==================================\n");

    // Dummy benchmark to satisfy criterion
    c.bench_function("compression_analysis", |b| {
        let encoder = JxlEncoder::new(EncoderOptions::default().quality(90.0));
        b.iter(|| {
            let mut encoded = Vec::new();
            encoder.encode(black_box(&image), Cursor::new(&mut encoded)).unwrap();
            black_box(encoded.len());
        });
    });
}

/// Benchmark context model building
fn benchmark_context_model(c: &mut Criterion) {
    let mut group = c.benchmark_group("context_model");

    // Generate sample coefficients
    let mut coefficients = Vec::new();
    for block in 0..100 {
        // DC coefficient
        coefficients.push((block % 128) as i16);
        // AC coefficients (mostly zeros with some values)
        for i in 1..64 {
            if i < 10 && block % 3 == 0 {
                coefficients.push(((i + block) % 16) as i16);
            } else {
                coefficients.push(0);
            }
        }
    }

    group.throughput(Throughput::Elements(coefficients.len() as u64));

    group.bench_function("build_from_coefficients", |b| {
        b.iter(|| {
            let model = ContextModel::build_from_coefficients(black_box(&coefficients)).unwrap();
            black_box(model);
        });
    });

    group.finish();
}

criterion_group!(
    benches,
    benchmark_compression_ratio,
    benchmark_encode_quality_levels,
    benchmark_decode_speed,
    benchmark_roundtrip,
    benchmark_context_model
);
criterion_main!(benches);
