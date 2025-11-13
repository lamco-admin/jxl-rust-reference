//! Benchmarks for rANS entropy coding
//!
//! Run with: cargo bench --bench entropy_coding

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jxl_bitstream::ans::{AnsDistribution, RansEncoder, RansDecoder};

fn bench_rans_small_alphabet(c: &mut Criterion) {
    let mut group = c.benchmark_group("rANS Small Alphabet");

    // 4 symbols (like DC coefficients)
    let frequencies = vec![100, 200, 300, 400];
    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
    let symbols: Vec<usize> = (0..1000).map(|i| i % 4).collect();

    group.bench_function("encode_4_symbols_1000_items", |b| {
        b.iter(|| {
            let mut encoder = RansEncoder::new();
            for &sym in symbols.iter().rev() {
                encoder.encode_symbol(black_box(sym), black_box(&dist)).unwrap();
            }
            encoder.finalize()
        });
    });

    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }
    let encoded = encoder.finalize();

    group.bench_function("decode_4_symbols_1000_items", |b| {
        b.iter(|| {
            let mut decoder = RansDecoder::new(black_box(encoded.clone())).unwrap();
            let mut decoded = Vec::with_capacity(symbols.len());
            for _ in 0..symbols.len() {
                decoded.push(decoder.decode_symbol(black_box(&dist)).unwrap());
            }
            decoded
        });
    });

    group.finish();
}

fn bench_rans_medium_alphabet(c: &mut Criterion) {
    let mut group = c.benchmark_group("rANS Medium Alphabet");

    // 16 symbols
    let frequencies: Vec<u32> = (1..=16).map(|i| i * 100).collect();
    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
    let symbols: Vec<usize> = (0..1000).map(|i| i % 16).collect();

    group.bench_function("encode_16_symbols_1000_items", |b| {
        b.iter(|| {
            let mut encoder = RansEncoder::new();
            for &sym in symbols.iter().rev() {
                encoder.encode_symbol(black_box(sym), black_box(&dist)).unwrap();
            }
            encoder.finalize()
        });
    });

    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }
    let encoded = encoder.finalize();

    group.bench_function("decode_16_symbols_1000_items", |b| {
        b.iter(|| {
            let mut decoder = RansDecoder::new(black_box(encoded.clone())).unwrap();
            let mut decoded = Vec::with_capacity(symbols.len());
            for _ in 0..symbols.len() {
                decoded.push(decoder.decode_symbol(black_box(&dist)).unwrap());
            }
            decoded
        });
    });

    group.finish();
}

fn bench_rans_large_alphabet(c: &mut Criterion) {
    let mut group = c.benchmark_group("rANS Large Alphabet");

    // 270 symbols (like AC coefficients)
    let frequencies: Vec<u32> = (1..=270).map(|i| ((i * 17) % 500) + 10).collect();
    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
    let symbols: Vec<usize> = (0..1000).map(|i| (i * 7) % 270).collect();

    group.bench_function("encode_270_symbols_1000_items", |b| {
        b.iter(|| {
            let mut encoder = RansEncoder::new();
            for &sym in symbols.iter().rev() {
                encoder.encode_symbol(black_box(sym), black_box(&dist)).unwrap();
            }
            encoder.finalize()
        });
    });

    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }
    let encoded = encoder.finalize();

    group.bench_function("decode_270_symbols_1000_items", |b| {
        b.iter(|| {
            let mut decoder = RansDecoder::new(black_box(encoded.clone())).unwrap();
            let mut decoded = Vec::with_capacity(symbols.len());
            for _ in 0..symbols.len() {
                decoded.push(decoder.decode_symbol(black_box(&dist)).unwrap());
            }
            decoded
        });
    });

    group.finish();
}

fn bench_rans_distribution_building(c: &mut Criterion) {
    let mut group = c.benchmark_group("rANS Distribution Building");

    // Small alphabet
    let small_freqs: Vec<u32> = vec![100, 200, 300, 400];
    group.bench_function("build_distribution_4_symbols", |b| {
        b.iter(|| {
            AnsDistribution::from_frequencies(black_box(&small_freqs)).unwrap()
        });
    });

    // Large alphabet
    let large_freqs: Vec<u32> = (1..=270).map(|i| ((i * 17) % 500) + 10).collect();
    group.bench_function("build_distribution_270_symbols", |b| {
        b.iter(|| {
            AnsDistribution::from_frequencies(black_box(&large_freqs)).unwrap()
        });
    });

    group.finish();
}

fn bench_rans_by_alphabet_size(c: &mut Criterion) {
    let mut group = c.benchmark_group("rANS Encode by Alphabet Size");

    for alphabet_size in [4, 8, 16, 32, 64, 128, 256].iter() {
        let frequencies: Vec<u32> = (1..=*alphabet_size).map(|i| ((i * 17) % 500) + 10).collect();
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
        let symbols: Vec<usize> = (0..1000).map(|i| i % alphabet_size).collect();

        group.bench_with_input(
            BenchmarkId::from_parameter(alphabet_size),
            alphabet_size,
            |b, _| {
                b.iter(|| {
                    let mut encoder = RansEncoder::new();
                    for &sym in symbols.iter().rev() {
                        encoder.encode_symbol(black_box(sym), black_box(&dist)).unwrap();
                    }
                    encoder.finalize()
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_rans_small_alphabet,
    bench_rans_medium_alphabet,
    bench_rans_large_alphabet,
    bench_rans_distribution_building,
    bench_rans_by_alphabet_size
);
criterion_main!(benches);
