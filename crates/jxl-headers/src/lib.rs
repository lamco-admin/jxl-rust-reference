//! JPEG XL header parsing and generation

pub mod animation;
pub mod container;

use jxl_bitstream::BitReader;
use jxl_core::*;
use std::io::Read;

pub use animation::{Animation, AnimationHeader, BlendMode, FrameHeader};
pub use container::{Container, JxlBox, BoxType, CONTAINER_SIGNATURE, CODESTREAM_SIGNATURE};

/// JPEG XL file header
#[derive(Debug, Clone)]
pub struct JxlHeader {
    pub version: u32,
    pub dimensions: Dimensions,
    pub bit_depth: u8,
    pub num_channels: usize,
    pub color_encoding: ColorEncoding,
    pub orientation: Orientation,
    pub is_animation: bool,
    pub have_preview: bool,
}

impl JxlHeader {
    /// Parse header from bitstream
    pub fn parse<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        // Read signature
        let signature = reader.read_bits(16)? as u16;
        if signature != 0x0AFF {
            return Err(JxlError::InvalidSignature);
        }

        // Read size header
        let size_header = reader.read_bits(8)? as u8;
        let small_size = (size_header & 0b11) == 0;

        let (width, height) = if small_size {
            let w = reader.read_bits(5)? as u32 + 1;
            let h = reader.read_bits(5)? as u32 + 1;
            (w, h)
        } else {
            let w = reader.read_u32(9)?;
            let h = reader.read_u32(9)?;
            (w, h)
        };

        // Read bit depth
        let bit_depth_enc = reader.read_bits(2)? as u8;
        let bit_depth = match bit_depth_enc {
            0 => 8,
            1 => 10,
            2 => 12,
            3 => reader.read_bits(6)? as u8 + 1,
            _ => unreachable!(),
        };

        // Read number of channels
        let num_extra = reader.read_bits(2)? as usize;
        let num_channels = 3 + num_extra;

        // Read color encoding
        let color_enc = reader.read_bits(2)? as u8;
        let color_encoding = match color_enc {
            0 => ColorEncoding::SRGB,
            1 => ColorEncoding::LinearSRGB,
            2 => ColorEncoding::XYB,
            3 => ColorEncoding::Custom,
            _ => unreachable!(),
        };

        // Read orientation
        let orientation_bits = reader.read_bits(3)? as u8;
        let orientation = match orientation_bits {
            1 => Orientation::Identity,
            2 => Orientation::FlipHorizontal,
            3 => Orientation::Rotate180,
            _ => Orientation::Identity,
        };

        // Read flags
        let is_animation = reader.read_bit()?;
        let have_preview = reader.read_bit()?;

        Ok(Self {
            version: 0,
            dimensions: Dimensions::new(width, height),
            bit_depth,
            num_channels,
            color_encoding,
            orientation,
            is_animation,
            have_preview,
        })
    }
}
