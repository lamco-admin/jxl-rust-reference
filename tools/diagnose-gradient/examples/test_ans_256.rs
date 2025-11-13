// Test if rANS breaks specifically at symbol >= 256

use jxl_bitstream::{RansEncoder, RansDecoder, AnsDistribution};

fn test_range(min_sym: usize, max_sym: usize) {
    let alphabet_size = max_sym + 1;
    let mut frequencies = vec![1u32; alphabet_size];
    
    // Give test symbols higher frequency
    for sym in min_sym..=max_sym {
        frequencies[sym] = 10;
    }

    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();
    
    let symbols: Vec<usize> = (min_sym..=max_sym).collect();
    
    // Encode
    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }
    let encoded = encoder.finalize();
    let encoded_len = encoded.len();

    // Decode
    let mut decoder = RansDecoder::new(encoded).unwrap();
    let mut decoded = Vec::new();
    for _ in 0..symbols.len() {
        decoded.push(decoder.decode_symbol(&dist).unwrap());
    }

    let matches = symbols == decoded;
    println!("Range [{}..{}]: {} symbols, {} bytes, {}",
             min_sym, max_sym, symbols.len(), encoded_len,
             if matches { "✓ OK" } else { "✗ FAIL" });
    
    if !matches {
        println!("  Expected: {:?}", &symbols[0..5.min(symbols.len())]);
        println!("  Got:      {:?}", &decoded[0..5.min(decoded.len())]);
    }
}

fn main() {
    println!("Testing rANS with different symbol ranges:\n");
    
    test_range(0, 10);      // Small range
    test_range(100, 110);   // Mid range
    test_range(250, 260);   // Around 256 boundary
    test_range(0, 255);     // Exactly 256 symbols
    test_range(0, 256);     // 257 symbols
    test_range(0, 269);     // Full range like AC coefficients
}
