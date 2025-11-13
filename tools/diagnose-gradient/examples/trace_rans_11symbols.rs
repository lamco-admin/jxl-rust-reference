use jxl_bitstream::ans::{AnsDistribution, RansEncoder, RansDecoder};

fn main() {
    println!("=== Tracing rANS Bug with 11 Symbols ===\n");

    // Create uniform distribution with 11 symbols
    let frequencies = vec![100u32; 11]; // Uniform frequencies
    let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

    println!("Distribution:");
    for i in 0..11 {
        let sym = dist.get_symbol(i).unwrap();
        println!("  Symbol {}: cumul={}, freq={}", i, sym.cumul, sym.freq);
    }
    println!();

    // Encode all 11 symbols: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
    let symbols: Vec<usize> = (0..=10).collect();
    println!("Encoding symbols (in reverse): {:?}\n", symbols);

    let mut encoder = RansEncoder::new();
    println!("Initial encoder state: {}\n", encoder.get_state());

    // Encode in reverse order (rANS is LIFO)
    for &sym in symbols.iter().rev() {
        let state_before = encoder.get_state();
        let sym_info = dist.get_symbol(sym).unwrap();
        println!("Encoding symbol {}: state_before={}, freq={}, cumul={}",
                 sym, state_before, sym_info.freq, sym_info.cumul);

        encoder.encode_symbol(sym, &dist).unwrap();

        let state_after = encoder.get_state();
        println!("  state_after={}", state_after);

        // Calculate what the state should be using the formula
        let q = state_before / sym_info.freq;
        let r = state_before % sym_info.freq;
        let expected_state = (q << 12) + r + sym_info.cumul;
        println!("  q={}, r={}, expected_state={}", q, r, expected_state);

        if state_after != expected_state {
            println!("  WARNING: state mismatch! (renormalization happened)");
        }
        println!();
    }

    let encoded = encoder.finalize();
    println!("Encoded {} bytes: {:?}\n", encoded.len(), encoded);

    // Decode
    println!("Decoding symbols:\n");
    let mut decoder = RansDecoder::new(encoded).unwrap();
    println!("Initial decoder state: {}\n", decoder.get_state());

    let mut decoded = Vec::new();
    for i in 0..symbols.len() {
        let state_before = decoder.get_state();
        let slot = state_before & (4096 - 1);
        let symbol_idx = decoder.peek_symbol(&dist).unwrap();
        let sym_info = dist.get_symbol(symbol_idx).unwrap();

        println!("Decoding position {}: state_before={}, slot={}", i, state_before, slot);
        println!("  lookup: symbol={}, freq={}, cumul={}", symbol_idx, sym_info.freq, sym_info.cumul);

        let symbol = decoder.decode_symbol(&dist).unwrap();
        decoded.push(symbol);

        let state_after = decoder.get_state();
        println!("  decoded symbol={}, state_after={}", symbol, state_after);

        // Calculate what state should be
        let quot = state_before >> 12;
        let rem = state_before & (4096 - 1);
        let expected_state = sym_info.freq * quot + rem - sym_info.cumul;
        println!("  quot={}, rem={}, expected_state={}", quot, rem, expected_state);

        if state_after != expected_state {
            println!("  WARNING: state mismatch! (renormalization happened)");
        }
        println!();
    }

    println!("Expected: {:?}", symbols);
    println!("Decoded:  {:?}", decoded);
    println!("Match: {}", symbols == decoded);
}
