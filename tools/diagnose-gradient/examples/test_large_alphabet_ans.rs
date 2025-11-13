// Test rANS with large alphabet like actual AC coefficients

use jxl_bitstream::{RansEncoder, RansDecoder, AnsDistribution};

fn main() {
    println!("Testing rANS with large alphabet (similar to AC coefficients)\n");

    // Create distribution with alphabet up to 270 (to cover symbol 269)
    let mut frequencies = vec![0u32; 270];
    // Symbols that appear in our actual data
    frequencies[125] = 10;  // -63
    frequencies[269] = 10;  // -135
    frequencies[36] = 10;   // 18
    frequencies[51] = 10;   // -26
    frequencies[52] = 10;   // 26
    frequencies[9] = 10;    // -5
    frequencies[4] = 10;    // 2
    frequencies[5] = 10;    // -3
    frequencies[15] = 10;   // -8
    frequencies[6] = 10;    // 3
    
    // Add base frequency to avoid zero frequencies
    for i in 0..270 {
        if frequencies[i] == 0 {
            frequencies[i] = 1;
        }
    }

    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
    println!("Created distribution with alphabet size {}", dist.alphabet_size());

    // Test symbols: exactly what Channel 1 should encode
    let symbols = vec![125usize, 269, 36, 51, 52, 9, 4, 5, 15, 6];
    println!("Original symbols: {:?}", symbols);

    // Encode in REVERSE
    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }

    let encoded = encoder.finalize();
    println!("Encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = RansDecoder::new(encoded).unwrap();
    let mut decoded = Vec::new();
    for _ in 0..symbols.len() {
        let sym = decoder.decode_symbol(&dist).unwrap();
        decoded.push(sym);
    }

    println!("Decoded symbols: {:?}", decoded);

    if decoded == symbols {
        println!("\n✓ Large alphabet rANS works!");
    } else {
        println!("\n✗ Large alphabet rANS BROKEN!");
    }
}
