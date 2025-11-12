//! JPEG XL decoder implementation

use jxl_bitstream::BitReader;
use jxl_core::*;
use jxl_headers::JxlHeader;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// JPEG XL decoder
pub struct JxlDecoder {
    header: Option<JxlHeader>,
}

impl JxlDecoder {
    pub fn new() -> Self {
        Self { header: None }
    }

    /// Decode a JPEG XL file from a path
    pub fn decode_file<P: AsRef<Path>>(&mut self, path: P) -> JxlResult<Image> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        self.decode(reader)
    }

    /// Decode from a reader
    pub fn decode<R: Read>(&mut self, reader: R) -> JxlResult<Image> {
        let mut bit_reader = BitReader::new(reader);

        // Parse header
        let header = JxlHeader::parse(&mut bit_reader)?;
        self.header = Some(header.clone());

        // Determine pixel type based on bit depth
        let pixel_type = if header.bit_depth <= 8 {
            PixelType::U8
        } else if header.bit_depth <= 16 {
            PixelType::U16
        } else {
            PixelType::F32
        };

        // Determine channels
        let channels = match header.num_channels {
            1 => ColorChannels::Gray,
            2 => ColorChannels::GrayAlpha,
            3 => ColorChannels::RGB,
            4 => ColorChannels::RGBA,
            _ => {
                return Err(JxlError::UnsupportedFeature(format!(
                    "{} channels not supported",
                    header.num_channels
                )))
            }
        };

        // Create image buffer
        let mut image = Image::new(
            header.dimensions,
            channels,
            pixel_type,
            header.color_encoding,
        )?;

        // Decode frame data
        self.decode_frame(&mut bit_reader, &mut image)?;

        Ok(image)
    }

    fn decode_frame<R: Read>(&self, reader: &mut BitReader<R>, image: &mut Image) -> JxlResult<()> {
        let header = self.header.as_ref().unwrap();

        // For this reference implementation, we'll decode a simplified version
        // A full implementation would handle:
        // - DC groups (2048x2048 regions)
        // - AC groups (256x256 regions)
        // - ANS entropy decoding
        // - Inverse DCT
        // - Color space conversion from XYB to RGB
        // - Dequantization

        // Simplified decoding: read raw pixel data
        // In reality, JPEG XL uses complex entropy coding and transforms
        let pixel_count = header.dimensions.pixel_count();
        let channel_count = header.num_channels;

        // Note: Using explicit indexing for clarity in this reference implementation
        #[allow(clippy::needless_range_loop)]
        match &mut image.buffer {
            ImageBuffer::U8(ref mut buffer) => {
                for i in 0..(pixel_count * channel_count) {
                    buffer[i] = reader.read_bits(8)? as u8;
                }
            }
            ImageBuffer::U16(ref mut buffer) => {
                for i in 0..(pixel_count * channel_count) {
                    buffer[i] = reader.read_bits(16)? as u16;
                }
            }
            ImageBuffer::F32(ref mut buffer) => {
                for i in 0..(pixel_count * channel_count) {
                    let bits = reader.read_bits(32)?;
                    buffer[i] = f32::from_bits(bits as u32);
                }
            }
        }

        Ok(())
    }

    /// Get the decoded header
    pub fn header(&self) -> Option<&JxlHeader> {
        self.header.as_ref()
    }
}

impl Default for JxlDecoder {
    fn default() -> Self {
        Self::new()
    }
}
