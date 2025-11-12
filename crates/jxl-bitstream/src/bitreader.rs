//! Bitstream reader implementation

use jxl_core::{JxlError, JxlResult};
use std::io::Read;

/// A bitstream reader for reading individual bits from a byte stream
pub struct BitReader<R: Read> {
    reader: R,
    buffer: u64,
    bits_in_buffer: usize,
}

impl<R: Read> BitReader<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            buffer: 0,
            bits_in_buffer: 0,
        }
    }

    /// Read up to 64 bits from the stream
    pub fn read_bits(&mut self, num_bits: usize) -> JxlResult<u64> {
        if num_bits > 64 {
            return Err(JxlError::InvalidParameter(
                "Cannot read more than 64 bits at once".to_string(),
            ));
        }

        // Ensure we have enough bits in the buffer
        while self.bits_in_buffer < num_bits {
            let mut byte = [0u8; 1];
            if self.reader.read(&mut byte)? == 0 {
                return Err(JxlError::InvalidBitstream(
                    "Unexpected end of stream".to_string(),
                ));
            }
            self.buffer |= (byte[0] as u64) << self.bits_in_buffer;
            self.bits_in_buffer += 8;
        }

        // Extract the requested bits
        let mask = if num_bits == 64 {
            u64::MAX
        } else {
            (1u64 << num_bits) - 1
        };
        let result = self.buffer & mask;
        self.buffer >>= num_bits;
        self.bits_in_buffer -= num_bits;

        Ok(result)
    }

    /// Read a single bit
    pub fn read_bit(&mut self) -> JxlResult<bool> {
        self.read_bits(1).map(|b| b != 0)
    }

    /// Read a variable-length integer (u32)
    pub fn read_u32(&mut self, selector: u32) -> JxlResult<u32> {
        let n = self.read_bits(selector as usize)? as u32;
        if n < (1 << selector) - 1 {
            Ok(n)
        } else {
            let extra_bits = self.read_bits(4)? as u32;
            let extra_value = self.read_bits(extra_bits as usize)? as u32;
            Ok((1 << selector) - 1 + extra_value)
        }
    }

    /// Skip to byte boundary
    pub fn align_to_byte(&mut self) -> JxlResult<()> {
        let bits_to_skip = self.bits_in_buffer % 8;
        if bits_to_skip > 0 {
            self.read_bits(bits_to_skip)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_read_bits() {
        let data = vec![0b10101010, 0b11001100];
        let mut reader = BitReader::new(Cursor::new(data));

        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert_eq!(reader.read_bits(4).unwrap(), 0b1010);
        assert_eq!(reader.read_bits(8).unwrap(), 0b11001100);
    }

    #[test]
    fn test_read_bit() {
        let data = vec![0b10101010];
        let mut reader = BitReader::new(Cursor::new(data));

        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
        assert!(!reader.read_bit().unwrap());
        assert!(reader.read_bit().unwrap());
    }
}
