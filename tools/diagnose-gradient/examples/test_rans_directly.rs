// Test rANS encoder/decoder directly

use jxl_bitstream::{RansEncoder, RansDecoder, AnsDistribution};

fn main() {
    println!("Testing rANS encoder/decoder directly\n");

    // Create a simple distribution
    let frequencies = vec![10u32, 20, 30, 15, 25]; // 5 symbols
    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

    // Test data: encode symbols [0, 1, 2, 3, 4, 0, 1, 2]
    let symbols = vec![0usize, 1, 2, 3, 4, 0, 1, 2];

    println!("Original symbols: {:?}", symbols);

    // Encode in REVERSE order (for rANS LIFO)
    let mut encoder = RansEncoder::new();
    for &sym in symbols.iter().rev() {
        encoder.encode_symbol(sym, &dist).unwrap();
    }

    let encoded_data = encoder.finalize();
    println!("Encoded to {} bytes", encoded_data.len());

    // Decode
    let mut decoder = RansDecoder::new(encoded_data).unwrap();
    let mut decoded = Vec::new();
    for _ in 0..symbols.len() {
        let sym = decoder.decode_symbol(&dist).unwrap();
        decoded.push(sym);
    }

    println!("Decoded symbols: {:?}", decoded);

    // Check if they match
    if decoded == symbols {
        println!("\n✓ rANS round-trip perfect!");
    } else {
        println!("\n✗ rANS BROKEN!");
        for i in 0..symbols.len() {
            if decoded[i] != symbols[i] {
                println!("  Position {}: {} -> {}", i, symbols[i], decoded[i]);
            }
        }
    }
}
