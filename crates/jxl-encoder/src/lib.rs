//! JPEG XL encoder implementation

use jxl_bitstream::BitWriter;
use jxl_core::*;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

/// Encoder options
#[derive(Debug, Clone)]
pub struct EncoderOptions {
    /// Quality (0-100, higher is better)
    pub quality: f32,
    /// Encoding effort (1-9, higher is slower but better compression)
    pub effort: u8,
    /// Use lossless encoding
    pub lossless: bool,
    /// Target bits per pixel (for lossy)
    pub target_bpp: Option<f32>,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            quality: consts::DEFAULT_QUALITY,
            effort: consts::DEFAULT_EFFORT,
            lossless: false,
            target_bpp: None,
        }
    }
}

impl EncoderOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(consts::MIN_QUALITY, consts::MAX_QUALITY);
        self
    }

    pub fn effort(mut self, effort: u8) -> Self {
        self.effort = effort.clamp(consts::MIN_EFFORT, consts::MAX_EFFORT);
        self
    }

    pub fn lossless(mut self, lossless: bool) -> Self {
        self.lossless = lossless;
        self
    }
}

/// JPEG XL encoder
pub struct JxlEncoder {
    /// Encoder configuration options
    /// Note: In this reference implementation, options are stored but not fully utilized yet.
    /// A complete implementation would use these for quality/effort trade-offs.
    #[allow(dead_code)]
    options: EncoderOptions,
}

impl JxlEncoder {
    pub fn new(options: EncoderOptions) -> Self {
        Self { options }
    }

    /// Encode an image to a file
    pub fn encode_file<P: AsRef<Path>>(&self, image: &Image, path: P) -> JxlResult<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.encode(image, writer)
    }

    /// Encode an image to a writer
    pub fn encode<W: Write>(&self, image: &Image, writer: W) -> JxlResult<()> {
        let mut bit_writer = BitWriter::new(writer);

        // Write signature
        bit_writer.write_bits(0x0AFF, 16)?;

        // Write size header (simplified)
        let small = image.width() <= 32 && image.height() <= 32;
        bit_writer.write_bits(if small { 0 } else { 1 }, 8)?;

        if small {
            bit_writer.write_bits((image.width() - 1) as u64, 5)?;
            bit_writer.write_bits((image.height() - 1) as u64, 5)?;
        } else {
            bit_writer.write_u32(image.width(), 9)?;
            bit_writer.write_u32(image.height(), 9)?;
        }

        // Write bit depth
        let bit_depth_enc = match image.pixel_type {
            PixelType::U8 => 0,
            PixelType::U16 => 2,
            PixelType::F16 => 2,
            PixelType::F32 => 3,
        };
        bit_writer.write_bits(bit_depth_enc, 2)?;
        if bit_depth_enc == 3 {
            bit_writer.write_bits(31, 6)?; // 32-bit
        }

        // Write channels
        let num_extra = image.channel_count() - 3;
        bit_writer.write_bits(num_extra as u64, 2)?;

        // Write color encoding
        let color_enc = match image.color_encoding {
            ColorEncoding::SRGB => 0,
            ColorEncoding::LinearSRGB => 1,
            ColorEncoding::XYB => 2,
            _ => 3,
        };
        bit_writer.write_bits(color_enc, 2)?;

        // Write orientation
        bit_writer.write_bits(1, 3)?; // Identity

        // Write flags
        bit_writer.write_bit(false)?; // not animation
        bit_writer.write_bit(false)?; // no preview

        // Encode frame data
        self.encode_frame(image, &mut bit_writer)?;

        bit_writer.flush()?;
        Ok(())
    }

    fn encode_frame<W: Write>(&self, image: &Image, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // For this reference implementation, we encode a simplified version
        // A full implementation would:
        // - Convert RGB to XYB color space
        // - Apply DCT transformation
        // - Quantize coefficients
        // - Encode using ANS entropy coding
        // - Group into DC/AC groups for parallel processing

        // Simplified encoding: write raw pixel data
        match &image.buffer {
            ImageBuffer::U8(buffer) => {
                for &pixel in buffer.iter() {
                    writer.write_bits(pixel as u64, 8)?;
                }
            }
            ImageBuffer::U16(buffer) => {
                for &pixel in buffer.iter() {
                    writer.write_bits(pixel as u64, 16)?;
                }
            }
            ImageBuffer::F32(buffer) => {
                for &pixel in buffer.iter() {
                    writer.write_bits(pixel.to_bits() as u64, 32)?;
                }
            }
        }

        Ok(())
    }
}

impl Default for JxlEncoder {
    fn default() -> Self {
        Self::new(EncoderOptions::default())
    }
}
