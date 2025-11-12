//! Bitstream writer implementation

use jxl_core::{JxlError, JxlResult};
use std::io::Write;

/// A bitstream writer for writing individual bits to a byte stream
pub struct BitWriter<W: Write> {
    writer: W,
    buffer: u64,
    bits_in_buffer: usize,
}

impl<W: Write> BitWriter<W> {
    pub fn new(writer: W) -> Self {
        Self {
            writer,
            buffer: 0,
            bits_in_buffer: 0,
        }
    }

    /// Write up to 64 bits to the stream
    pub fn write_bits(&mut self, value: u64, num_bits: usize) -> JxlResult<()> {
        if num_bits > 64 {
            return Err(JxlError::InvalidParameter(
                "Cannot write more than 64 bits at once".to_string(),
            ));
        }

        let mask = if num_bits == 64 {
            u64::MAX
        } else {
            (1u64 << num_bits) - 1
        };
        self.buffer |= (value & mask) << self.bits_in_buffer;
        self.bits_in_buffer += num_bits;

        // Flush complete bytes
        while self.bits_in_buffer >= 8 {
            self.writer.write_all(&[(self.buffer & 0xFF) as u8])?;
            self.buffer >>= 8;
            self.bits_in_buffer -= 8;
        }

        Ok(())
    }

    /// Write a single bit
    pub fn write_bit(&mut self, value: bool) -> JxlResult<()> {
        self.write_bits(value as u64, 1)
    }

    /// Write a variable-length integer (u32)
    pub fn write_u32(&mut self, value: u32, selector: u32) -> JxlResult<()> {
        let max_direct = (1 << selector) - 1;
        if value < max_direct {
            self.write_bits(value as u64, selector as usize)
        } else {
            self.write_bits(max_direct as u64, selector as usize)?;
            let extra = value - max_direct;
            let extra_bits = if extra == 0 {
                0
            } else {
                32 - extra.leading_zeros()
            };
            self.write_bits(extra_bits as u64, 4)?;
            self.write_bits(extra as u64, extra_bits as usize)
        }
    }

    /// Align to byte boundary by writing zero bits
    pub fn align_to_byte(&mut self) -> JxlResult<()> {
        let bits_to_write = (8 - (self.bits_in_buffer % 8)) % 8;
        if bits_to_write > 0 {
            self.write_bits(0, bits_to_write)?;
        }
        Ok(())
    }

    /// Flush remaining bits and the underlying writer
    pub fn flush(&mut self) -> JxlResult<()> {
        if self.bits_in_buffer > 0 {
            self.writer.write_all(&[(self.buffer & 0xFF) as u8])?;
            self.buffer = 0;
            self.bits_in_buffer = 0;
        }
        self.writer.flush()?;
        Ok(())
    }
}

impl<W: Write> Drop for BitWriter<W> {
    fn drop(&mut self) {
        let _ = self.flush();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_write_bits() {
        let mut output = Vec::new();
        let mut writer = BitWriter::new(Cursor::new(&mut output));

        writer.write_bits(0b1010, 4).unwrap();
        writer.write_bits(0b1010, 4).unwrap();
        writer.write_bits(0b11001100, 8).unwrap();
        writer.flush().unwrap();

        assert_eq!(output, vec![0b10101010, 0b11001100]);
    }

    #[test]
    fn test_write_bit() {
        let mut output = Vec::new();
        let mut writer = BitWriter::new(Cursor::new(&mut output));

        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.write_bit(false).unwrap();
        writer.write_bit(true).unwrap();
        writer.flush().unwrap();

        assert_eq!(output, vec![0b10101010]);
    }
}
