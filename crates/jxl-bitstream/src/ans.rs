//! Asymmetric Numeral Systems (ANS) entropy coding
//!
//! Production-grade ANS implementation for JPEG XL.
//! This implements tANS (table ANS) which is simpler and proven.

use jxl_core::{JxlError, JxlResult};
use std::collections::HashMap;

/// ANS table size (2^12 = 4096) - JPEG XL standard
pub const ANS_TAB_SIZE: u32 = 4096;
const ANS_LOG_TAB_SIZE: u32 = 12;

/// ANS lower bound for state (from libjxl)
const ANS_L: u32 = 1 << 16;  // 65536

/// ANS signature for initial/final state (from libjxl)
const ANS_SIGNATURE: u32 = 0x13;

/// ANS symbol with cumulative frequency and frequency
#[derive(Debug, Clone, Copy)]
pub struct Symbol {
    /// Cumulative frequency (start position in CDF)
    pub cumul: u32,
    /// Symbol frequency
    pub freq: u32,
}

/// ANS distribution for a set of symbols
#[derive(Debug, Clone)]
pub struct AnsDistribution {
    /// Symbol table indexed by symbol value
    symbols: Vec<Symbol>,
    /// Lookup table for decoding
    decode_table: Vec<usize>,
    /// Total frequency (should equal ANS_TAB_SIZE)
    total_freq: u32,
    /// Alphabet size
    alphabet_size: usize,
}

impl AnsDistribution {
    /// Create a new ANS distribution from symbol frequencies
    pub fn from_frequencies(frequencies: &[u32]) -> JxlResult<Self> {
        if frequencies.is_empty() {
            return Err(JxlError::InvalidParameter(
                "Empty frequency table".to_string(),
            ));
        }

        let alphabet_size = frequencies.len();
        let total: u32 = frequencies.iter().sum();

        if total == 0 {
            return Err(JxlError::InvalidParameter(
                "Sum of frequencies is zero".to_string(),
            ));
        }

        // Normalize frequencies to ANS_TAB_SIZE
        let mut normalized_freqs = vec![0u32; alphabet_size];
        let mut normalized_total = 0u32;

        // First pass: compute normalized frequencies
        for (i, &freq) in frequencies.iter().enumerate() {
            if freq > 0 {
                let normalized =
                    ((freq as u64 * ANS_TAB_SIZE as u64 + total as u64 / 2) / total as u64) as u32;
                normalized_freqs[i] = normalized.max(1); // Ensure non-zero symbols get at least 1
                normalized_total += normalized_freqs[i];
            }
        }

        // Second pass: adjust to exactly ANS_TAB_SIZE
        if normalized_total != ANS_TAB_SIZE {
            let max_idx = normalized_freqs
                .iter()
                .enumerate()
                .filter(|(_, &f)| f > 0)
                .max_by_key(|(_, &f)| f)
                .map(|(i, _)| i)
                .unwrap_or(0);

            let diff = normalized_total as i64 - ANS_TAB_SIZE as i64;
            normalized_freqs[max_idx] =
                (normalized_freqs[max_idx] as i64 - diff).max(1) as u32;
        }

        // Build cumulative distribution
        let mut symbols = Vec::with_capacity(alphabet_size);
        let mut cumul = 0u32;

        for &freq in &normalized_freqs {
            symbols.push(Symbol { cumul, freq });
            cumul += freq;
        }

        // Build decode table
        let mut decode_table = vec![0usize; ANS_TAB_SIZE as usize];
        for (symbol, sym) in symbols.iter().enumerate() {
            if sym.freq > 0 {
                for slot in sym.cumul..(sym.cumul + sym.freq) {
                    decode_table[slot as usize] = symbol;
                }
            }
        }

        Ok(Self {
            symbols,
            decode_table,
            total_freq: ANS_TAB_SIZE,
            alphabet_size,
        })
    }

    /// Create a uniform distribution
    pub fn uniform(alphabet_size: usize) -> JxlResult<Self> {
        if alphabet_size == 0 {
            return Err(JxlError::InvalidParameter(
                "Alphabet size must be > 0".to_string(),
            ));
        }

        let freq_per_symbol = ANS_TAB_SIZE / alphabet_size as u32;
        let frequencies = vec![freq_per_symbol.max(1); alphabet_size];
        Self::from_frequencies(&frequencies)
    }

    /// Get symbol information
    pub fn get_symbol(&self, symbol: usize) -> JxlResult<Symbol> {
        if symbol >= self.alphabet_size {
            return Err(JxlError::InvalidParameter(format!(
                "Symbol {} out of alphabet range {}",
                symbol, self.alphabet_size
            )));
        }
        Ok(self.symbols[symbol])
    }

    /// Get the total frequency (should equal ANS_TAB_SIZE)
    pub fn total_freq(&self) -> u32 {
        self.total_freq
    }

    /// Find symbol from slot (for decoding)
    fn find_symbol_from_slot(&self, slot: u32) -> usize {
        self.decode_table[slot as usize % (ANS_TAB_SIZE as usize)]
    }
}

/// Simple tANS encoder
pub struct RansEncoder {
    state: u32,
    output: Vec<u8>,
}

impl RansEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Self {
            state: ANS_SIGNATURE << 16,  // Initialize with ANS_SIGNATURE shifted
            output: Vec::new(),
        }
    }

    /// Encode a symbol using rANS (matching libjxl implementation)
    pub fn encode_symbol(&mut self, symbol: usize, dist: &AnsDistribution) -> JxlResult<()> {
        let sym = dist.get_symbol(symbol)?;

        // libjxl renormalization: check if upper bits exceed frequency
        // Condition: (state >> (32 - ANS_LOG_TAB_SIZE)) >= freq
        // This is equivalent to: state >= (freq << (32 - ANS_LOG_TAB_SIZE))
        while (self.state >> (32 - ANS_LOG_TAB_SIZE)) >= sym.freq {
            // Write lower 16 bits (libjxl writes 16 bits at a time, not 8)
            self.output.push((self.state & 0xFF) as u8);
            self.output.push(((self.state >> 8) & 0xFF) as u8);
            self.state >>= 16;
        }

        // rANS C step (from Duda's paper)
        // C(s,x) = (x / freq_s) * M + (x mod freq_s) + cumul_s
        let q = self.state / sym.freq;
        let r = self.state % sym.freq;
        self.state = (q << ANS_LOG_TAB_SIZE) + r + sym.cumul;

        Ok(())
    }

    /// Finalize encoding
    pub fn finalize(mut self) -> Vec<u8> {
        // Reverse renormalization bytes for decoding (LIFO order)
        // CRITICAL: Reverse in 16-bit chunks, not byte-by-byte!
        // We write 16 bits (2 bytes) at a time, so reverse in pairs
        assert!(self.output.len() % 2 == 0, "Output should be even number of bytes");

        let mut reversed = Vec::with_capacity(self.output.len());
        for chunk in self.output.chunks_exact(2).rev() {
            reversed.push(chunk[0]);
            reversed.push(chunk[1]);
        }

        // Prepend final state (4 bytes, little-endian)
        let mut result = Vec::with_capacity(reversed.len() + 4);
        result.push((self.state & 0xFF) as u8);
        result.push(((self.state >> 8) & 0xFF) as u8);
        result.push(((self.state >> 16) & 0xFF) as u8);
        result.push(((self.state >> 24) & 0xFF) as u8);
        result.extend_from_slice(&reversed);

        result
    }
}

impl Default for RansEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple tANS decoder
pub struct RansDecoder {
    state: u32,
    input: Vec<u8>,
    pos: usize,
}

impl RansDecoder {
    /// Create a new decoder
    pub fn new(input: Vec<u8>) -> JxlResult<Self> {
        if input.len() < 4 {
            return Err(JxlError::InvalidBitstream(
                "Insufficient data for ANS decoder".to_string(),
            ));
        }

        // Read initial state (4 bytes, little-endian)
        let state = input[0] as u32
            | ((input[1] as u32) << 8)
            | ((input[2] as u32) << 16)
            | ((input[3] as u32) << 24);

        Ok(Self {
            state,
            input,
            pos: 4,
        })
    }

    /// Decode a symbol (matching libjxl implementation)
    pub fn decode_symbol(&mut self, dist: &AnsDistribution) -> JxlResult<usize> {
        // Get symbol from current state
        let slot = self.state & (ANS_TAB_SIZE - 1);
        let symbol = dist.find_symbol_from_slot(slot);
        let sym = dist.symbols[symbol];

        // Update state
        self.state = sym.freq * (self.state >> ANS_LOG_TAB_SIZE)
            + (self.state & (ANS_TAB_SIZE - 1))
            - sym.cumul;

        // Renormalize: read 16 bits at a time (matching libjxl)
        // Threshold: state < ANS_L (65536)
        if self.state < ANS_L {
            if self.pos + 1 >= self.input.len() {
                return Err(JxlError::InvalidBitstream(
                    "Unexpected end of ANS stream".to_string(),
                ));
            }
            // Read 16 bits (2 bytes) little-endian
            let bits = self.input[self.pos] as u32 | ((self.input[self.pos + 1] as u32) << 8);
            self.state = (self.state << 16) | bits;
            self.pos += 2;
        }

        Ok(symbol)
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.pos >= self.input.len()
    }
}

/// Build frequency distribution from data
pub fn build_distribution(data: &[i16]) -> AnsDistribution {
    if data.is_empty() {
        return AnsDistribution::uniform(256).unwrap();
    }

    // Count symbol frequencies
    let mut freq_map: HashMap<i16, u32> = HashMap::new();
    let mut min_val = i16::MAX;
    let mut max_val = i16::MIN;

    for &val in data {
        *freq_map.entry(val).or_insert(0) += 1;
        min_val = min_val.min(val);
        max_val = max_val.max(val);
    }

    // Map to 0-based alphabet
    let range = (max_val - min_val + 1) as usize;
    let mut frequencies = vec![0u32; range];

    for (&val, &freq) in &freq_map {
        let idx = (val - min_val) as usize;
        frequencies[idx] = freq;
    }

    // Create distribution
    AnsDistribution::from_frequencies(&frequencies)
        .unwrap_or_else(|_| AnsDistribution::uniform(range).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ans_distribution_uniform() {
        let dist = AnsDistribution::uniform(256).unwrap();
        assert_eq!(dist.alphabet_size, 256);
        assert_eq!(dist.total_freq(), ANS_TAB_SIZE);
    }

    #[test]
    fn test_ans_distribution_from_frequencies() {
        let frequencies = vec![100, 200, 300, 400];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        assert_eq!(dist.alphabet_size, 4);
        assert_eq!(dist.total_freq(), ANS_TAB_SIZE);
    }

    #[test]
    fn test_rans_encode_decode_simple() {
        let frequencies = vec![1000, 2000, 1000];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        println!("\n=== SIMPLE TEST ===");
        println!("Distribution:");
        for (i, sym) in dist.symbols.iter().enumerate() {
            println!("  Symbol {}: cumul={}, freq={}", i, sym.cumul, sym.freq);
        }

        let symbols = vec![0, 1, 2, 1, 0];

        // Encode in REVERSE order (ANS is LIFO - Last In First Out)
        let mut encoder = RansEncoder::new();
        println!("\nEncoding in reverse order:");
        for &sym in symbols.iter().rev() {
            let state_before = encoder.state;
            encoder.encode_symbol(sym, &dist).unwrap();
            println!("  Symbol {}: {} -> {} (renorm threshold: {})",
                sym, state_before, encoder.state,
                dist.symbols[sym].freq << (32 - ANS_LOG_TAB_SIZE));
        }

        let encoded = encoder.finalize();
        println!("\nEncoded {} bytes: {:?}", encoded.len(), &encoded[..encoded.len().min(20)]);
        println!("Final state in bytes: [{}, {}, {}, {}]",
            encoded[0], encoded[1], encoded[2], encoded[3]);

        // Decode (will come out in forward order)
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();
        println!("\nDecoding (initial state: {}, ANS_L: {}):", decoder.state, ANS_L);

        for i in 0..symbols.len() {
            let state_before = decoder.state;
            let slot = state_before & (ANS_TAB_SIZE - 1);
            let sym = decoder.decode_symbol(&dist).unwrap();
            println!("  [{}] slot: {}, symbol: {}, state: {} -> {}",
                i, slot, sym, state_before, decoder.state);
            decoded.push(sym);
        }

        println!("\nExpected: {:?}", symbols);
        println!("Got:      {:?}", decoded);
        assert_eq!(symbols, decoded);
    }

    #[test]
    fn test_rans_encode_decode_complex() {
        let frequencies = vec![100, 200, 300, 400, 500, 300, 200];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        println!("\nComplex test - Distribution:");
        for (i, sym) in dist.symbols.iter().enumerate() {
            println!("  Symbol {}: cumul={}, freq={}", i, sym.cumul, sym.freq);
        }

        let symbols = vec![0, 1, 2, 3, 4, 5, 6, 4, 3, 2, 1, 0];

        // Encode in REVERSE order (ANS is LIFO - Last In First Out)
        let mut encoder = RansEncoder::new();
        println!("\nEncoding in reverse:");
        for &sym in symbols.iter().rev() {
            let state_before = encoder.state;
            encoder.encode_symbol(sym, &dist).unwrap();
            println!("  Symbol {}: {} -> {}", sym, state_before, encoder.state);
        }

        let encoded = encoder.finalize();
        println!("Encoded {} bytes: {:?}", encoded.len(), &encoded[..encoded.len().min(20)]);

        // Decode (will come out in forward order)
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();

        println!("\nDecoding (initial state: {}):", decoder.state);
        for i in 0..symbols.len() {
            let state_before = decoder.state;
            let slot = state_before & (ANS_TAB_SIZE - 1);
            let sym = decoder.decode_symbol(&dist).unwrap();
            println!("  [{}] State: {} -> {}, slot: {}, symbol: {}", i, state_before, decoder.state, slot, sym);
            decoded.push(sym);
        }

        println!("Expected: {:?}", symbols);
        println!("Got:      {:?}", decoded);
        assert_eq!(symbols, decoded);
    }

    #[test]
    fn test_build_distribution() {
        let data = vec![1, 2, 3, 2, 1, 0, -1, 0, 1, 2];
        let dist = build_distribution(&data);

        assert!(dist.alphabet_size > 0);
        assert_eq!(dist.total_freq(), ANS_TAB_SIZE);
    }

    #[test]
    fn test_rans_minimal_renorm() {
        // Minimal test: 2 symbols, force renormalization
        println!("\n=== MINIMAL RENORM TEST ===");

        // Use low frequency to force renormalization quickly
        let frequencies = vec![3896, 200];  // Symbol 1 has low freq = 200
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        println!("Distribution:");
        for (i, sym) in dist.symbols.iter().enumerate() {
            println!("  Symbol {}: cumul={}, freq={}", i, sym.cumul, sym.freq);
        }

        // Encode many symbols to force state buildup and renormalization
        let symbols = vec![1; 50];  // 50 high-frequency symbols

        let mut encoder = RansEncoder::new();
        println!("\nEncoding (initial state: {}):", encoder.state);
        for &sym in symbols.iter().rev() {
            let state_before = encoder.state;
            let threshold = dist.symbols[sym].freq << (32 - ANS_LOG_TAB_SIZE);
            let will_renorm = (state_before >> (32 - ANS_LOG_TAB_SIZE)) >= dist.symbols[sym].freq;
            encoder.encode_symbol(sym, &dist).unwrap();
            println!("  Symbol {}: state {} -> {} (threshold: {}, renorm: {})",
                sym, state_before, encoder.state, threshold, will_renorm);
        }

        let encoded = encoder.finalize();
        println!("\nEncoded {} bytes: {:?}", encoded.len(), encoded);

        // Decode
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();
        println!("\nDecoding (initial state: {}):", decoder.state);

        for i in 0..symbols.len() {
            let state_before = decoder.state;
            let slot = state_before & (ANS_TAB_SIZE - 1);
            let sym = decoder.decode_symbol(&dist).unwrap();
            let did_renorm = decoder.state > state_before; // State increased = renorm happened
            println!("  [{}] slot: {}, symbol: {}, state: {} -> {} (renorm: {})",
                i, slot, sym, state_before, decoder.state, did_renorm);
            decoded.push(sym);
        }

        println!("\nExpected: {:?}", symbols);
        println!("Got:      {:?}", decoded);
        assert_eq!(symbols, decoded);
    }
}
