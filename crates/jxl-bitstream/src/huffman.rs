//! Huffman coding support (used for certain JPEG XL components)

use jxl_core::{JxlError, JxlResult};

/// Huffman tree node
#[derive(Debug, Clone)]
enum HuffmanNode {
    Leaf(u32),
    Internal(Box<HuffmanNode>, Box<HuffmanNode>),
}

/// Huffman decoder
#[derive(Debug)]
pub struct HuffmanDecoder {
    root: Option<HuffmanNode>,
}

impl HuffmanDecoder {
    pub fn new() -> Self {
        Self { root: None }
    }

    /// Build Huffman tree from code lengths
    pub fn build_from_lengths(&mut self, code_lengths: &[u8]) -> JxlResult<()> {
        // Build canonical Huffman codes
        let max_length = *code_lengths.iter().max().unwrap_or(&0) as usize;
        if max_length == 0 {
            return Ok(());
        }

        // Count symbols for each code length
        let mut length_counts = vec![0u32; max_length + 1];
        for &length in code_lengths {
            if length > 0 {
                length_counts[length as usize] += 1;
            }
        }

        // Calculate starting codes for each length
        let mut next_code = vec![0u32; max_length + 1];
        let mut code = 0u32;
        for i in 1..=max_length {
            code = (code + length_counts[i - 1]) << 1;
            next_code[i] = code;
        }

        // Assign codes to symbols
        let mut codes = Vec::new();
        for (symbol, &length) in code_lengths.iter().enumerate() {
            if length > 0 {
                codes.push((symbol as u32, length, next_code[length as usize]));
                next_code[length as usize] += 1;
            }
        }

        // Build tree from codes
        self.root = Some(HuffmanNode::Internal(
            Box::new(HuffmanNode::Leaf(0)),
            Box::new(HuffmanNode::Leaf(0)),
        ));

        for (symbol, length, code) in codes {
            self.insert_code(symbol, length, code)?;
        }

        Ok(())
    }

    fn insert_code(&mut self, symbol: u32, length: u8, code: u32) -> JxlResult<()> {
        let mut node = self.root.as_mut().unwrap();

        for i in (0..length).rev() {
            let bit = (code >> i) & 1;
            node = match node {
                HuffmanNode::Internal(left, right) => {
                    if bit == 0 {
                        left
                    } else {
                        right
                    }
                }
                HuffmanNode::Leaf(_) => {
                    return Err(JxlError::InvalidBitstream(
                        "Invalid Huffman tree".to_string(),
                    ));
                }
            };
        }

        *node = HuffmanNode::Leaf(symbol);
        Ok(())
    }

    /// Decode a symbol from bits
    pub fn decode<F>(&self, read_bit: &mut F) -> JxlResult<u32>
    where
        F: FnMut() -> JxlResult<bool>,
    {
        let mut node = self.root.as_ref().ok_or_else(|| {
            JxlError::InvalidBitstream("Huffman tree not initialized".to_string())
        })?;

        loop {
            match node {
                HuffmanNode::Leaf(symbol) => return Ok(*symbol),
                HuffmanNode::Internal(left, right) => {
                    let bit = read_bit()?;
                    node = if bit { right } else { left };
                }
            }
        }
    }
}

impl Default for HuffmanDecoder {
    fn default() -> Self {
        Self::new()
    }
}
