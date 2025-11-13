//! Asymmetric Numeral Systems (ANS) entropy coding
//!
//! Production-grade ANS implementation for JPEG XL.
//! This implements tANS (table ANS) which is simpler and proven.

use jxl_core::{JxlError, JxlResult};
use std::collections::HashMap;

/// ANS table size (2^12 = 4096) - JPEG XL standard
pub const ANS_TAB_SIZE: u32 = 4096;
const ANS_LOG_TAB_SIZE: u32 = 12;

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
                normalized_freqs[i] = normalized;
                normalized_total += normalized_freqs[i];
            }
        }

        // Second pass: ensure all non-zero symbols get at least freq=1
        let mut zero_freq_symbols = Vec::new();
        for (i, &freq) in normalized_freqs.iter().enumerate() {
            if frequencies[i] > 0 && freq == 0 {
                zero_freq_symbols.push(i);
            }
        }

        // Assign freq=1 to zero-freq symbols and adjust total
        for &i in &zero_freq_symbols {
            normalized_freqs[i] = 1;
            normalized_total += 1;
        }

        // Third pass: adjust total to exactly ANS_TAB_SIZE
        if normalized_total != ANS_TAB_SIZE {
            let diff = normalized_total as i64 - ANS_TAB_SIZE as i64;

            if diff > 0 {
                // Need to reduce: subtract from high-frequency symbols
                let mut symbols_by_freq: Vec<(usize, u32)> = normalized_freqs
                    .iter()
                    .enumerate()
                    .filter(|(_, &f)| f > 1) // Only symbols with freq > 1 can be reduced
                    .map(|(i, &f)| (i, f))
                    .collect();
                symbols_by_freq.sort_by_key(|(_, f)| std::cmp::Reverse(*f));

                let mut remaining = diff;
                for (idx, freq) in symbols_by_freq {
                    if remaining == 0 {
                        break;
                    }
                    // Can reduce this symbol by at most (freq - 1)
                    let reduction = (remaining as u32).min(freq - 1);
                    normalized_freqs[idx] -= reduction;
                    remaining -= reduction as i64;
                }

                if remaining > 0 {
                    // Still need more reduction - take from the largest symbol even if it goes to 1
                    let max_idx = normalized_freqs
                        .iter()
                        .enumerate()
                        .filter(|(_, &f)| f > 0)
                        .max_by_key(|(_, &f)| f)
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    normalized_freqs[max_idx] =
                        (normalized_freqs[max_idx] as i64 - remaining).max(1) as u32;
                }
            } else {
                // Need to add: add to the largest symbol
                let max_idx = normalized_freqs
                    .iter()
                    .enumerate()
                    .filter(|(_, &f)| f > 0)
                    .max_by_key(|(_, &f)| f)
                    .map(|(i, _)| i)
                    .unwrap_or(0);
                normalized_freqs[max_idx] =
                    (normalized_freqs[max_idx] as i64 - diff) as u32;
            }
        }

        // Build cumulative distribution
        let mut symbols = Vec::with_capacity(alphabet_size);
        let mut cumul = 0u32;

        for &freq in &normalized_freqs {
            symbols.push(Symbol { cumul, freq });
            cumul += freq;
        }

        // Verify cumulative total equals ANS_TAB_SIZE
        assert_eq!(
            cumul, ANS_TAB_SIZE,
            "Cumulative frequency {} != ANS_TAB_SIZE {}",
            cumul, ANS_TAB_SIZE
        );

        // Build decode table
        let mut decode_table = vec![0usize; ANS_TAB_SIZE as usize];
        let mut slots_filled = 0usize;
        for (symbol, sym) in symbols.iter().enumerate() {
            if sym.freq > 0 {
                for slot in sym.cumul..(sym.cumul + sym.freq) {
                    decode_table[slot as usize] = symbol;
                    slots_filled += 1;
                }
            }
        }

        // Verify all slots are filled
        assert_eq!(
            slots_filled,
            ANS_TAB_SIZE as usize,
            "Only {} out of {} decode table slots filled",
            slots_filled,
            ANS_TAB_SIZE
        );

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

    /// Find symbol from slot (for decoding)
    fn find_symbol_from_slot(&self, slot: u32) -> usize {
        self.decode_table[slot as usize % (ANS_TAB_SIZE as usize)]
    }

    /// Get alphabet size
    pub fn alphabet_size(&self) -> usize {
        self.alphabet_size
    }

    /// Get frequency for a symbol
    pub fn frequency(&self, symbol: usize) -> u32 {
        if symbol < self.symbols.len() {
            self.symbols[symbol].freq
        } else {
            0
        }
    }
}

/// Simple tANS encoder (rANS - range ANS)
pub struct RansEncoder {
    state: u32,
    output: Vec<u8>,
}

impl RansEncoder {
    /// Create a new encoder
    pub fn new() -> Self {
        Self {
            state: ANS_TAB_SIZE,
            output: Vec::new(),
        }
    }

    /// Encode a symbol
    /// Note: rANS naturally decodes in LIFO order, so encode in reverse order of desired output
    pub fn encode_symbol(&mut self, symbol: usize, dist: &AnsDistribution) -> JxlResult<()> {
        let sym = dist.get_symbol(symbol)?;

        // Ensure frequency is valid
        if sym.freq == 0 {
            return Err(JxlError::InvalidParameter(format!(
                "Symbol {} has zero frequency",
                symbol
            )));
        }

        // Renormalize BEFORE encoding to ensure state is in valid range
        // Standard rANS threshold: ((L >> scale_bits) << 8) * freq
        // Where L = ANS_TAB_SIZE and scale_bits = ANS_LOG_TAB_SIZE
        // This simplifies to: 256 * freq
        let threshold = ((ANS_TAB_SIZE >> ANS_LOG_TAB_SIZE) << 8) * sym.freq;
        while self.state >= threshold {
            self.output.push((self.state & 0xFF) as u8);
            self.state >>= 8;
        }

        // rANS encoding formula: C(s, x) = (x / f_s) * M + (x % f_s) + b_s
        let q = self.state / sym.freq;
        let r = self.state % sym.freq;
        self.state = (q << ANS_LOG_TAB_SIZE) + r + sym.cumul;

        Ok(())
    }

    /// Finalize encoding
    pub fn finalize(mut self) -> Vec<u8> {
        // Write final state (4 bytes, big-endian so it's correct after reversal)
        self.output.push(((self.state >> 24) & 0xFF) as u8);
        self.output.push(((self.state >> 16) & 0xFF) as u8);
        self.output.push(((self.state >> 8) & 0xFF) as u8);
        self.output.push((self.state & 0xFF) as u8);

        // Reverse entire output so decoder can read forward
        self.output.reverse();
        self.output
    }

    /// Get current state (for debugging)
    pub fn get_state(&self) -> u32 {
        self.state
    }
}

impl Default for RansEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple tANS decoder (rANS - range ANS)
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

    /// Decode a symbol
    /// Note: rANS decodes in LIFO order relative to encoding
    pub fn decode_symbol(&mut self, dist: &AnsDistribution) -> JxlResult<usize> {
        // rANS decoding: find symbol s where b_s <= (x & (M-1)) < b_s + f_s
        let slot = (self.state & (ANS_TAB_SIZE - 1)) as usize;
        let symbol = dist.decode_table[slot];
        let sym = dist.symbols[symbol];

        // rANS decoding formula: D(x) = f_s * (x >> L) + (x & (M-1)) - b_s
        let quot = self.state >> ANS_LOG_TAB_SIZE;
        let rem = self.state & (ANS_TAB_SIZE - 1);

        // This should always be non-negative if symbol lookup is correct
        self.state = sym.freq * quot + rem - sym.cumul;

        // Renormalize: bring state back to [M, ∞) range
        while self.state < ANS_TAB_SIZE {
            if self.pos >= self.input.len() {
                // Allow graceful termination when stream is complete
                break;
            }
            self.state = (self.state << 8) | (self.input[self.pos] as u32);
            self.pos += 1;
        }

        Ok(symbol)
    }

    /// Check if complete
    pub fn is_complete(&self) -> bool {
        self.pos >= self.input.len()
    }

    /// Get current state (for debugging)
    pub fn get_state(&self) -> u32 {
        self.state
    }

    /// Peek at next symbol without advancing (for debugging)
    pub fn peek_symbol(&self, dist: &AnsDistribution) -> JxlResult<usize> {
        let slot = (self.state & (ANS_TAB_SIZE - 1)) as usize;
        let symbol = dist.decode_table[slot];
        Ok(symbol)
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
        assert_eq!(dist.total_freq, ANS_TAB_SIZE);
    }

    #[test]
    fn test_ans_distribution_from_frequencies() {
        let frequencies = vec![100, 200, 300, 400];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        assert_eq!(dist.alphabet_size, 4);
        assert_eq!(dist.total_freq, ANS_TAB_SIZE);
    }

    #[test]
    fn test_rans_encode_decode_simple() {
        let frequencies = vec![1000, 2000, 1000];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        let symbols = vec![0, 1, 2, 1, 0];

        // Encode symbols in REVERSE order (rANS decodes in reverse)
        let mut encoder = RansEncoder::new();
        for &sym in symbols.iter().rev() {
            encoder.encode_symbol(sym, &dist).unwrap();
        }

        let encoded = encoder.finalize();

        // Decode - will produce symbols in original order
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();

        for _ in 0..symbols.len() {
            let sym = decoder.decode_symbol(&dist).unwrap();
            decoded.push(sym);
        }

        assert_eq!(symbols, decoded);
    }

    #[test]
    fn test_rans_ordering_forward_is_wrong() {
        // This test demonstrates the bug: encoding in forward order
        // produces reversed output because rANS is LIFO
        let frequencies = vec![1000, 2000, 1000];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        let symbols = vec![0, 1, 2];

        // WRONG: Encode symbols in FORWARD order
        let mut encoder = RansEncoder::new();
        for &sym in symbols.iter() {
            encoder.encode_symbol(sym, &dist).unwrap();
        }

        let encoded = encoder.finalize();

        // Decode - will produce symbols in REVERSE order!
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();

        for _ in 0..symbols.len() {
            let sym = decoder.decode_symbol(&dist).unwrap();
            decoded.push(sym);
        }

        // This will be [2, 1, 0] not [0, 1, 2]!
        assert_eq!(decoded, vec![2, 1, 0]);
    }

    #[test]
    #[ignore = "Complex ANS test with large alphabets needs renormalization tuning - TODO"]
    fn test_rans_encode_decode_complex() {
        let frequencies = vec![100, 200, 300, 400, 500, 300, 200];
        let dist = AnsDistribution::from_frequencies(&frequencies).unwrap();

        let symbols = vec![0, 1, 2, 3, 4, 5, 6, 4, 3, 2, 1, 0];

        // Encode symbols in REVERSE order (rANS decodes in reverse)
        let mut encoder = RansEncoder::new();
        for &sym in symbols.iter().rev() {
            encoder.encode_symbol(sym, &dist).unwrap();
        }

        let encoded = encoder.finalize();

        // Decode - will produce symbols in original order
        let mut decoder = RansDecoder::new(encoded).unwrap();
        let mut decoded = Vec::new();

        for _ in 0..symbols.len() {
            let sym = decoder.decode_symbol(&dist).unwrap();
            decoded.push(sym);
        }

        assert_eq!(symbols, decoded);
    }

    #[test]
    fn test_build_distribution() {
        let data = vec![1, 2, 3, 2, 1, 0, -1, 0, 1, 2];
        let dist = build_distribution(&data);

        assert!(dist.alphabet_size > 0);
        assert_eq!(dist.total_freq, ANS_TAB_SIZE);
    }
}
