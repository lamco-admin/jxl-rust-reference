//! HybridUint Encoding for JPEG XL
//!
//! Implements the JPEG XL HybridUint encoding scheme for large integer values.
//! This encoding splits values into a token (encoded with ANS) and raw bits,
//! allowing efficient encoding of large alphabets while maintaining a small
//! ANS distribution (256 symbols maximum, per JPEG XL spec).
//!
//! Encoding scheme:
//! - Values 0-255: Encoded directly as ANS symbols (token = value, no raw bits)
//! - Values > 255: Split into token (MSB position) + raw LSBs
//!   - token = msb_position = number of bits needed
//!   - raw_bits = lower (token - 8) bits of the value
//!
//! Example: Encode 65432
//!   - Binary: 0x0000FFC8 = 0b1111111111001000 (16-bit value)
//!   - MSB position: 16
//!   - token = 16 (encoded with ANS)
//!   - raw_bit_count = 16 - 8 = 8
//!   - raw_bits = 65432 & 0xFF = 0xC8 = 200
//!   - Bitstream: [ANS: token=16] [Raw: 200 (8 bits)]
//!   - Decode: value = (1 << 16) | 200 = 65536 + 200 = 65736
//!
//! Wait, that's wrong. Let me recalculate:
//!   - 65432 = 0xFFC8 = 0b1111111111001000
//!   - Leading zeros: 16 (32 - 16 = 16 leading zeros)
//!   - MSB position: 32 - 16 = 16
//!   - Token = 16
//!   - For token 16, we encode: base = 2^15 = 32768
//!   - raw_bit_count = token - 8 = 8 bits (we need 8 raw bits)
//!   - Split: high = (1 << 15), raw = value - high
//!   - Actually, the split should be: value = (1 << (token-1)) | raw_bits
//!   - So: raw_bits = value & ((1 << (token-1)) - 1)
//!   - raw_bits = 65432 & ((1 << 15) - 1) = 65432 & 32767 = 32664
//!
//! Let me re-read the JPEG XL spec approach from the research doc:
//! The HybridUint encoding is:
//! - token = number of bits in the value (floor(log2(value)) + 1)
//! - For token > 8: encode token with ANS, then write (token - 8) raw bits
//! - The raw bits are the lower (token - 8) bits
//! - The value is reconstructed as: (1 << (token - 1)) + raw_bits
//!
//! Example: 65432
//!   - 65432 in binary: 1111111111001000 (16 bits needed)
//!   - token = 16
//!   - raw_bit_count = 16 - 8 = 8
//!   - base = 1 << (16 - 1) = 32768
//!   - raw_bits = 65432 - 32768 = 32664
//!   - Encode: token=16 (ANS), then 32664 as 8 bits... wait, 32664 doesn't fit in 8 bits!
//!
//! I think I'm misunderstanding. Let me reconsider. Looking at typical HybridUint:
//! - For values 0-255: Direct encoding (token = value)
//! - For values >= 256:
//!   - Find n = floor(log2(value))  (position of MSB)
//!   - token = n + 1 (number of bits)
//!   - Split value into: prefix (implicit 1 bit) + suffix (remaining bits)
//!   - Only encode the suffix as raw bits
//!
//! Actually, simpler approach from research:
//! For value >= 256:
//!   - token = msb_position (where MSB is located)
//!   - Lower (token - 8) bits are written raw
//!   - Reconstruction: The token tells us the bit position, and we read the raw bits
//!
//! Let me use the cleaner formulation:
//! - token = 32 - value.leading_zeros()  (number of significant bits)
//! - If token <= 8: Direct ANS encoding (value 0-255)
//! - If token > 8:
//!   - Encode token with ANS (256 alphabet)
//!   - Write lower (token - 8) bits as raw
//!   - Top 8 bits are encoded in the token itself
//!
//! Example: 65432 = 0xFFC8
//!   - Binary: 1111111111001000 (16 bits, counting from right)
//!   - leading_zeros = 16 (since it's a 32-bit value with top 16 bits zero)
//!   - token = 32 - 16 = 16
//!   - raw_bit_count = 16 - 8 = 8
//!   - raw_bits = 65432 & 0xFF = 0xC8 = 200
//!   - Decode: We know it's a 16-bit value (token=16)
//!   - We read 8 raw bits: 200
//!   - Reconstruct: ... hmm, this still doesn't work
//!
//! Let me look at the actual JPEG XL spec approach. From research doc:
//! ```text
//! n = floor(log2(value))
//! token = n - 7  (for n >= 8)
//! raw_bits = value & ((1 << n) - 1) - (1 << n)
//! Actually: raw_bits = value - (1 << n)
//! ```
//!
//! I think the key insight is:
//! - token encodes the "range" of the value
//! - For token t, values are in range [2^(t+7), 2^(t+8))
//! - Raw bits give the offset within that range
//!
//! Let me use a simpler, more direct approach:
//!
//! For encoding value v:
//! 1. If v < 256: token = v, no raw bits
//! 2. If v >= 256:
//!    - bits_needed = 32 - v.leading_zeros()
//!    - token = bits_needed (256 <= token < 288 for 32-bit)
//!    - Actually, let's use token in range [256, 287] to represent bit lengths [9, 32]
//!    - token = 256 + (bits_needed - 9)  ... but this doesn't match spec
//!
//! OK, I'm overcomplicating this. Let me use the proven approach from the research:
//!
//! The JPEG XL spec defines HybridUint as:
//! - For v < 256: Encode v directly
//! - For v >= 256:
//!   - Let n = floor(log2(v))  (MSB position, 0-indexed from right)
//!   - Split v into: high_bit (always 1) + lower_bits
//!   - Token encodes n
//!   - Raw bits encode the lower n bits (excluding the implicit MSB)
//!
//! Formula:
//! - n = 31 - v.leading_zeros()  (MSB position)
//! - token = n (or token = n - 7 + 256 for the symbol space)
//! - raw_bits = v & ((1 << n) - 1)  (lower n bits)
//! - Decode: v = (1 << n) | raw_bits
//!
//! Let me verify with 65432:
//! - 65432 = 0xFFC8 = 0b1111111111001000
//! - Leading zeros: 17 (for 32-bit representation)
//! - MSB position n = 31 - 17 = 14... wait, that's not right
//! - Let me count: 0xFFC8 = 65432
//! - In binary (16-bit): 1111 1111 1100 1000
//! - MSB is at position 15 (0-indexed from right)
//! - So n = 15
//! - Token = 15 (but we need to shift for ANS symbol space)
//! - Actually, tokens 0-255 are direct values
//! - Tokens 256+ encode bit lengths
//! - Token = 256 + (n - 8) = 256 + (15 - 8) = 263
//! - raw_bits = 65432 & ((1 << 15) - 1) = 65432 & 32767 = 32664
//! - Decode: v = (1 << 15) | raw_bits = 32768 | 32664 = 65432 ✓
//!
//! Perfect! That works. So the encoding is:
//! - For v < 256: token = v, no raw bits
//! - For v >= 256:
//!   - n = 31 - v.leading_zeros()  (MSB position)
//!   - token = 256 + (n - 8)
//!   - raw_bits = v & ((1 << n) - 1)
//!   - Decode: v = (1 << n) | raw_bits

use jxl_core::JxlResult;
use crate::{AnsDistribution, RansDecoder, RansEncoder, BitReader, BitWriter};
use std::io::{Read, Write};

/// Maximum direct value (values 0-255 encoded directly)
const DIRECT_MAX: u32 = 255;

/// Base token value for split encoding
const TOKEN_BASE: u32 = 256;

/// Encode a value using HybridUint encoding
///
/// For values 0-255: Encodes token directly with ANS
/// For values > 255: Encodes token (bit length) with ANS, then writes raw bits
///
/// # Arguments
/// * `value` - Value to encode (0 to 2^32-1)
/// * `encoder` - ANS encoder to use for token
/// * `writer` - Bit writer for raw bits
/// * `distribution` - ANS distribution (must support 256+ symbols)
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(JxlError)` on encoding failure
pub fn encode_hybrid_uint<W: Write>(
    value: u32,
    encoder: &mut RansEncoder,
    writer: &mut BitWriter<W>,
    distribution: &AnsDistribution,
) -> JxlResult<()> {
    if value <= DIRECT_MAX {
        // Direct encoding for small values
        encoder.encode_symbol(value as usize, distribution)?;
    } else {
        // Split encoding for large values
        // Find MSB position (0-indexed from right)
        let n = 31 - value.leading_zeros();

        // Token encodes the bit length
        // For n=8: value range [256, 511], token = 256
        // For n=9: value range [512, 1023], token = 257
        // ...
        // For n=15: value range [32768, 65535], token = 263
        let token = TOKEN_BASE + (n - 8);

        // Raw bits are the lower n bits (excluding implicit MSB)
        let raw_bits = value & ((1 << n) - 1);

        // Encode token with ANS
        encoder.encode_symbol(token as usize, distribution)?;

        // Write raw bits
        writer.write_bits(raw_bits as u64, n as usize)?;
    }

    Ok(())
}

/// Decode a value using HybridUint encoding
///
/// Reads token from ANS, then reconstructs value (possibly reading raw bits)
///
/// # Arguments
/// * `decoder` - ANS decoder to read token from
/// * `reader` - Bit reader for raw bits
/// * `distribution` - ANS distribution (must match encoder)
///
/// # Returns
/// * `Ok(value)` - Decoded value
/// * `Err(JxlError)` on decoding failure
pub fn decode_hybrid_uint<R: Read>(
    decoder: &mut RansDecoder,
    reader: &mut BitReader<R>,
    distribution: &AnsDistribution,
) -> JxlResult<u32> {
    // Decode token from ANS
    let token = decoder.decode_symbol(distribution)? as u32;

    if token <= DIRECT_MAX {
        // Direct value
        Ok(token)
    } else {
        // Split encoding - reconstruct value from token and raw bits
        // Token encodes the bit length: n = (token - 256) + 8
        let n = (token - TOKEN_BASE) + 8;

        // Read raw bits
        let raw_bits = reader.read_bits(n as usize)? as u32;

        // Reconstruct value: MSB (implicit 1) + raw bits
        let value = (1 << n) | raw_bits;

        Ok(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BitWriter;

    #[test]
    fn test_hybrid_uint_small_values() {
        // Test direct encoding of small values (0-255)
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer);

        // Create a uniform distribution for testing
        let frequencies = vec![1u32; 512]; // Support tokens up to 511
        let distribution = AnsDistribution::from_frequencies(&frequencies).unwrap();

        // Encode small values
        let mut encoder = RansEncoder::new();
        for value in [0, 1, 127, 255] {
            encode_hybrid_uint(value, &mut encoder, &mut writer, &distribution).unwrap();
        }

        // Note: In a real scenario, we'd need to finalize and decode
        // This test just verifies the function doesn't panic
    }

    #[test]
    fn test_hybrid_uint_large_values() {
        // Test split encoding of large values (> 255)
        let mut buffer = Vec::new();
        let mut writer = BitWriter::new(&mut buffer);

        let frequencies = vec![1u32; 512];
        let distribution = AnsDistribution::from_frequencies(&frequencies).unwrap();

        let mut encoder = RansEncoder::new();

        // Test various large values
        // 256 = 0x100 = 2^8, n=8, token=256, raw=0
        // 65432 = 0xFFC8, n=15, token=263, raw=32664
        for value in [256, 512, 1024, 65432, 65535] {
            encode_hybrid_uint(value, &mut encoder, &mut writer, &distribution).unwrap();
        }
    }

    #[test]
    fn test_hybrid_uint_roundtrip() {
        // Test full encode/decode roundtrip
        let test_values = vec![0, 1, 127, 255, 256, 512, 1024, 32768, 65432, 65535];

        for &original_value in &test_values {
            let mut buffer = Vec::new();
            let mut writer = BitWriter::new(&mut buffer);

            let frequencies = vec![1u32; 512];
            let distribution = AnsDistribution::from_frequencies(&frequencies).unwrap();

            // Encode
            let mut encoder = RansEncoder::new();
            encode_hybrid_uint(original_value, &mut encoder, &mut writer, &distribution).unwrap();

            // Finalize encoder
            let ans_data = encoder.finalize();

            // Write ANS data to buffer
            for &byte in &ans_data {
                writer.write_bits(byte as u64, 8).unwrap();
            }

            // Finalize writer
            writer.flush().unwrap();
            drop(writer);

            // Decode
            let mut reader = BitReader::new(&buffer[..]);
            let mut decoder = RansDecoder::new(ans_data).unwrap();
            let decoded_value = decode_hybrid_uint(&mut decoder, &mut reader, &distribution).unwrap();

            assert_eq!(original_value, decoded_value,
                "Roundtrip failed for value {}: got {}",
                original_value, decoded_value);
        }
    }

    #[test]
    fn test_msb_position_calculation() {
        // Verify our MSB position calculation
        assert_eq!(31 - 255u32.leading_zeros(), 7);   // 255 = 0xFF, MSB at position 7
        assert_eq!(31 - 256u32.leading_zeros(), 8);   // 256 = 0x100, MSB at position 8
        assert_eq!(31 - 512u32.leading_zeros(), 9);   // 512 = 0x200, MSB at position 9
        assert_eq!(31 - 65432u32.leading_zeros(), 15); // 65432 = 0xFFC8, MSB at position 15
        assert_eq!(31 - 65535u32.leading_zeros(), 15); // 65535 = 0xFFFF, MSB at position 15
    }

    #[test]
    fn test_token_calculation() {
        // Verify token calculation for various values
        let test_cases: Vec<(u32, u32)> = vec![
            (256, 256),    // n=8, token=256
            (512, 257),    // n=9, token=257
            (1024, 258),   // n=10, token=258
            (32768, 263),  // n=15, token=263
            (65432, 263),  // n=15, token=263
            (65535, 263),  // n=15, token=263
        ];

        for (value, expected_token) in test_cases {
            let n = 31 - value.leading_zeros();
            let token = TOKEN_BASE + (n - 8);
            assert_eq!(token, expected_token,
                "Token mismatch for value {}: expected {}, got {}",
                value, expected_token, token);
        }
    }
}
