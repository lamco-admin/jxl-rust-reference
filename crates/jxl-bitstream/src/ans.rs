//! Asymmetric Numeral Systems (ANS) entropy coding
//!
//! ANS is the primary entropy coding method used in JPEG XL.
//! This module implements both ANS encoding and decoding.

use jxl_core::{JxlError, JxlResult};

/// ANS state size in bits
const ANS_LOG_TAB_SIZE: u32 = 12;
const ANS_TAB_SIZE: usize = 1 << ANS_LOG_TAB_SIZE; // 4096
const ANS_TAB_MASK: u32 = (ANS_TAB_SIZE - 1) as u32;

/// ANS distribution table entry
#[derive(Debug, Clone, Copy)]
pub struct AnsTableEntry {
    pub freq: u16,
    pub offset: u16,
}

/// ANS decoder
pub struct AnsDecoder {
    state: u32,
    table: Vec<AnsTableEntry>,
}

impl AnsDecoder {
    pub fn new() -> Self {
        Self {
            state: 0,
            table: Vec::new(),
        }
    }

    /// Initialize the decoder with a frequency table
    pub fn init_table(&mut self, frequencies: &[u32]) -> JxlResult<()> {
        if frequencies.is_empty() {
            return Err(JxlError::InvalidParameter(
                "Empty frequency table".to_string(),
            ));
        }

        // Normalize frequencies to ANS_TAB_SIZE
        let total: u32 = frequencies.iter().sum();
        if total == 0 {
            return Err(JxlError::InvalidParameter(
                "Sum of frequencies is zero".to_string(),
            ));
        }

        self.table.clear();
        self.table.resize(
            ANS_TAB_SIZE,
            AnsTableEntry {
                freq: 0,
                offset: 0,
            },
        );

        let mut pos = 0;
        for (symbol, &freq) in frequencies.iter().enumerate() {
            if freq == 0 {
                continue;
            }

            let normalized_freq = ((freq as u64 * ANS_TAB_SIZE as u64) / total as u64) as u16;
            let normalized_freq = normalized_freq.max(1);

            for _ in 0..normalized_freq {
                if pos >= ANS_TAB_SIZE {
                    break;
                }
                self.table[pos] = AnsTableEntry {
                    freq: normalized_freq,
                    offset: symbol as u16,
                };
                pos += 1;
            }
        }

        Ok(())
    }

    /// Set the initial state
    pub fn set_state(&mut self, state: u32) {
        self.state = state;
    }

    /// Decode a symbol and update state
    pub fn decode_symbol(&mut self, bits: &mut impl Iterator<Item = u32>) -> JxlResult<u32> {
        let index = (self.state & ANS_TAB_MASK) as usize;
        let entry = self.table[index];

        let symbol = entry.offset as u32;
        self.state = (entry.freq as u32) * (self.state >> ANS_LOG_TAB_SIZE);

        // Renormalize
        while self.state < ANS_TAB_SIZE as u32 {
            let bit = bits.next().ok_or_else(|| {
                JxlError::InvalidBitstream("Unexpected end of bitstream".to_string())
            })?;
            self.state = (self.state << 1) | bit;
        }

        Ok(symbol)
    }
}

impl Default for AnsDecoder {
    fn default() -> Self {
        Self::new()
    }
}

/// ANS encoder
pub struct AnsEncoder {
    state: u32,
    table: Vec<AnsTableEntry>,
}

impl AnsEncoder {
    pub fn new() -> Self {
        Self {
            state: ANS_TAB_SIZE as u32,
            table: Vec::new(),
        }
    }

    /// Initialize the encoder with a frequency table
    pub fn init_table(&mut self, frequencies: &[u32]) -> JxlResult<()> {
        if frequencies.is_empty() {
            return Err(JxlError::InvalidParameter(
                "Empty frequency table".to_string(),
            ));
        }

        // Normalize frequencies (same as decoder)
        let total: u32 = frequencies.iter().sum();
        if total == 0 {
            return Err(JxlError::InvalidParameter(
                "Sum of frequencies is zero".to_string(),
            ));
        }

        self.table.clear();
        self.table.resize(
            frequencies.len(),
            AnsTableEntry {
                freq: 0,
                offset: 0,
            },
        );

        let mut cumulative = 0u32;
        for (symbol, &freq) in frequencies.iter().enumerate() {
            let normalized_freq = ((freq as u64 * ANS_TAB_SIZE as u64) / total as u64) as u16;
            let normalized_freq = normalized_freq.max(1);

            self.table[symbol] = AnsTableEntry {
                freq: normalized_freq,
                offset: cumulative as u16,
            };
            cumulative += normalized_freq as u32;
        }

        Ok(())
    }

    /// Encode a symbol and emit bits
    pub fn encode_symbol(&mut self, symbol: u32) -> JxlResult<Vec<u32>> {
        if symbol >= self.table.len() as u32 {
            return Err(JxlError::InvalidParameter(format!(
                "Symbol {} out of range",
                symbol
            )));
        }

        let entry = self.table[symbol as usize];
        let mut bits = Vec::new();

        // Renormalize before encoding
        while self.state >= (ANS_TAB_SIZE as u32) * (entry.freq as u32) {
            bits.push(self.state & 1);
            self.state >>= 1;
        }

        // Update state
        self.state = (self.state / entry.freq as u32) * ANS_TAB_SIZE as u32
            + (self.state % entry.freq as u32)
            + entry.offset as u32;

        Ok(bits)
    }

    /// Get the final state
    pub fn get_state(&self) -> u32 {
        self.state
    }
}

impl Default for AnsEncoder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ans_encode_decode() {
        let frequencies = vec![100, 200, 300, 400];

        let mut encoder = AnsEncoder::new();
        encoder.init_table(&frequencies).unwrap();

        let symbols = vec![0, 1, 2, 3, 2, 1, 0];
        let mut all_bits = Vec::new();

        for &symbol in &symbols {
            let bits = encoder.encode_symbol(symbol).unwrap();
            all_bits.extend(bits);
        }

        let mut decoder = AnsDecoder::new();
        decoder.init_table(&frequencies).unwrap();
        decoder.set_state(encoder.get_state());

        let mut bit_iter = all_bits.into_iter().rev();
        let mut decoded = Vec::new();

        for _ in 0..symbols.len() {
            let symbol = decoder.decode_symbol(&mut bit_iter).unwrap();
            decoded.push(symbol);
        }

        decoded.reverse();
        assert_eq!(symbols, decoded);
    }
}
