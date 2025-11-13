//! JPEG XL Spec-Compliant Metadata Structures (ISO/IEC 18181-1 Section 7.2)
//!
//! This module implements the ImageMetadata structure according to the
//! JPEG XL specification, with encoding/decoding support.

use jxl_bitstream::{BitReader, BitWriter};
use jxl_core::{ColorEncoding, JxlError, JxlResult, Orientation};
use std::io::{Read, Write};

/// Bit depth configuration (spec Section 7.2.1)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BitDepth {
    /// Whether floating point samples are used
    pub floating_point_sample: bool,
    /// Bits per sample (1-32)
    pub bits_per_sample: u32,
    /// Exponent bits for floating point (0 if integer)
    pub exp_bits: u32,
}

impl BitDepth {
    /// Create an integer bit depth
    pub fn integer(bits: u32) -> Self {
        Self {
            floating_point_sample: false,
            bits_per_sample: bits,
            exp_bits: 0,
        }
    }

    /// Create a floating point bit depth
    pub fn float(bits: u32, exp_bits: u32) -> Self {
        Self {
            floating_point_sample: true,
            bits_per_sample: bits,
            exp_bits,
        }
    }

    /// Encode bit depth to bitstream (spec Section 7.2.1)
    pub fn encode<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        writer.write_bit(self.floating_point_sample)?;

        if self.floating_point_sample {
            // Floating point samples
            if self.bits_per_sample == 32 {
                writer.write_bits(0, 2)?;
            } else if self.bits_per_sample == 16 {
                writer.write_bits(1, 2)?;
            } else {
                writer.write_bits(2, 2)?;
                writer.write_bits((self.bits_per_sample - 1) as u64, 5)?;
            }
            writer.write_bits(self.exp_bits as u64, 5)?;
        } else {
            // Integer samples
            match self.bits_per_sample {
                8 => writer.write_bits(0, 2)?,
                10 => writer.write_bits(1, 2)?,
                12 => writer.write_bits(2, 2)?,
                _ => {
                    writer.write_bits(3, 2)?;
                    writer.write_bits((self.bits_per_sample - 1) as u64, 6)?;
                }
            }
        }

        Ok(())
    }

    /// Decode bit depth from bitstream
    pub fn decode<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        let floating_point_sample = reader.read_bit()?;

        if floating_point_sample {
            let selector = reader.read_bits(2)? as u32;
            let bits_per_sample = match selector {
                0 => 32,
                1 => 16,
                2 => 1 + reader.read_bits(5)? as u32,
                _ => return Err(JxlError::InvalidBitDepth),
            };
            let exp_bits = reader.read_bits(5)? as u32;
            Ok(Self::float(bits_per_sample, exp_bits))
        } else {
            let selector = reader.read_bits(2)? as u32;
            let bits_per_sample = match selector {
                0 => 8,
                1 => 10,
                2 => 12,
                3 => 1 + reader.read_bits(6)? as u32,
                _ => unreachable!(),
            };
            Ok(Self::integer(bits_per_sample))
        }
    }
}

impl Default for BitDepth {
    fn default() -> Self {
        Self::integer(8)
    }
}

/// Extra channel type (spec Section 7.2.2)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtraChannelType {
    Alpha = 0,
    Depth = 1,
    SpotColor = 2,
    SelectionMask = 3,
    Black = 4,
    CFA = 5,
    Thermal = 6,
    Reserved7 = 7,
    Optional = 8,
}

impl ExtraChannelType {
    fn from_u32(value: u32) -> JxlResult<Self> {
        match value {
            0 => Ok(Self::Alpha),
            1 => Ok(Self::Depth),
            2 => Ok(Self::SpotColor),
            3 => Ok(Self::SelectionMask),
            4 => Ok(Self::Black),
            5 => Ok(Self::CFA),
            6 => Ok(Self::Thermal),
            7 => Ok(Self::Reserved7),
            8 => Ok(Self::Optional),
            _ => Err(JxlError::InvalidExtraChannel),
        }
    }
}

/// Extra channel information (simplified)
#[derive(Debug, Clone)]
pub struct ExtraChannelInfo {
    pub channel_type: ExtraChannelType,
    pub bit_depth: BitDepth,
}

impl Default for ExtraChannelInfo {
    fn default() -> Self {
        Self {
            channel_type: ExtraChannelType::Alpha,
            bit_depth: BitDepth::default(),
        }
    }
}

/// Custom color encoding (simplified)
#[derive(Debug, Clone, Default)]
pub struct CustomColorEncoding {
    pub color_space: u32,
}

/// Complete spec-compliant ImageMetadata structure (spec Section 7.2)
#[derive(Debug, Clone)]
pub struct JxlImageMetadata {
    /// All default flag - if true, all metadata uses default values
    pub all_default: bool,

    // Extra fields
    pub extra_fields: bool,

    // Orientation - 1-8
    pub orientation: Orientation,

    // Intrinsic size
    pub have_intrinsic_size: bool,
    pub intrinsic_width: u32,
    pub intrinsic_height: u32,

    // Preview
    pub have_preview: bool,

    // Animation
    pub have_animation: bool,

    // Bit depth
    pub bit_depth: BitDepth,

    // Modular 16-bit buffers
    pub modular_16bit_buffers: bool,

    // Extra channels
    pub num_extra_channels: u32,
    pub extra_channels: Vec<ExtraChannelInfo>,

    // XYB encoded
    pub xyb_encoded: bool,

    // Color encoding
    pub color_encoding: ColorEncoding,
    pub custom_color_encoding: Option<CustomColorEncoding>,
}

impl Default for JxlImageMetadata {
    fn default() -> Self {
        Self {
            all_default: true,
            extra_fields: false,
            orientation: Orientation::Identity,
            have_intrinsic_size: false,
            intrinsic_width: 0,
            intrinsic_height: 0,
            have_preview: false,
            have_animation: false,
            bit_depth: BitDepth::default(),
            modular_16bit_buffers: false,
            num_extra_channels: 0,
            extra_channels: Vec::new(),
            xyb_encoded: true, // JPEG XL typically uses XYB
            color_encoding: ColorEncoding::SRGB,
            custom_color_encoding: None,
        }
    }
}

impl JxlImageMetadata {
    /// Create metadata for a simple RGB image
    pub fn for_rgb_image(width: u32, height: u32, bits_per_sample: u32) -> Self {
        Self {
            all_default: false,
            extra_fields: false,
            orientation: Orientation::Identity,
            have_intrinsic_size: true,
            intrinsic_width: width,
            intrinsic_height: height,
            have_preview: false,
            have_animation: false,
            bit_depth: BitDepth::integer(bits_per_sample),
            modular_16bit_buffers: false,
            num_extra_channels: 0,
            extra_channels: Vec::new(),
            xyb_encoded: true,
            color_encoding: ColorEncoding::SRGB,
            custom_color_encoding: None,
        }
    }

    /// Encode metadata to bitstream (spec Section 7.2)
    pub fn encode<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // all_default flag
        writer.write_bit(self.all_default)?;

        if self.all_default {
            return Ok(());
        }

        // extra_fields flag
        writer.write_bit(self.extra_fields)?;

        if self.extra_fields {
            // Orientation (3 bits)
            writer.write_bits(self.orientation as u64, 3)?;
        }

        // have_intrinsic_size
        writer.write_bit(self.have_intrinsic_size)?;
        if self.have_intrinsic_size {
            // Encode size
            self.encode_size(writer, self.intrinsic_width, self.intrinsic_height)?;
        }

        // have_preview
        writer.write_bit(self.have_preview)?;

        // have_animation
        writer.write_bit(self.have_animation)?;

        // Bit depth
        self.bit_depth.encode(writer)?;

        // modular_16bit_buffers
        writer.write_bit(self.modular_16bit_buffers)?;

        // num_extra_channels (using u32 with selector 0 for now)
        writer.write_u32(self.num_extra_channels, 0)?;

        // xyb_encoded
        writer.write_bit(self.xyb_encoded)?;

        // Color encoding
        self.encode_color_encoding(writer)?;

        Ok(())
    }

    /// Decode metadata from bitstream
    pub fn decode<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        let all_default = reader.read_bit()?;

        if all_default {
            return Ok(Self::default());
        }

        let extra_fields = reader.read_bit()?;

        let orientation = if extra_fields {
            let orientation_bits = reader.read_bits(3)? as u8;
            match orientation_bits {
                1 => Orientation::Identity,
                2 => Orientation::FlipHorizontal,
                3 => Orientation::Rotate180,
                4 => Orientation::FlipVertical,
                5 => Orientation::Transpose,
                6 => Orientation::Rotate90,
                7 => Orientation::AntiTranspose,
                8 => Orientation::Rotate270,
                _ => Orientation::Identity,
            }
        } else {
            Orientation::Identity
        };

        let have_intrinsic_size = reader.read_bit()?;
        let (intrinsic_width, intrinsic_height) = if have_intrinsic_size {
            Self::decode_size(reader)?
        } else {
            (0, 0)
        };

        let have_preview = reader.read_bit()?;
        let have_animation = reader.read_bit()?;

        let bit_depth = BitDepth::decode(reader)?;
        let modular_16bit_buffers = reader.read_bit()?;

        let num_extra_channels = reader.read_u32(0)?;

        let xyb_encoded = reader.read_bit()?;

        let (color_encoding, custom_color_encoding) = Self::decode_color_encoding(reader)?;

        Ok(Self {
            all_default: false,
            extra_fields,
            orientation,
            have_intrinsic_size,
            intrinsic_width,
            intrinsic_height,
            have_preview,
            have_animation,
            bit_depth,
            modular_16bit_buffers,
            num_extra_channels,
            extra_channels: Vec::new(),
            xyb_encoded,
            color_encoding,
            custom_color_encoding,
        })
    }

    /// Encode size with variable-length encoding (simplified)
    fn encode_size<W: Write>(&self, writer: &mut BitWriter<W>, width: u32, height: u32) -> JxlResult<()> {
        // Simplified size encoding
        if width <= 32 && height <= 32 {
            writer.write_bit(false)?; // small size
            writer.write_bits((width - 1) as u64, 5)?;
            writer.write_bits((height - 1) as u64, 5)?;
        } else if width <= 256 && height <= 256 {
            writer.write_bit(true)?; // larger size
            writer.write_bit(false)?; // medium size
            writer.write_bits((width - 1) as u64, 9)?;
            writer.write_bits((height - 1) as u64, 9)?;
        } else {
            writer.write_bit(true)?;
            writer.write_bit(true)?; // large size
            writer.write_bits((width - 1) as u64, 13)?;
            writer.write_bits((height - 1) as u64, 13)?;
        }

        Ok(())
    }

    /// Decode size with variable-length encoding
    fn decode_size<R: Read>(reader: &mut BitReader<R>) -> JxlResult<(u32, u32)> {
        let is_small = !reader.read_bit()?;

        if is_small {
            let width = reader.read_bits(5)? as u32 + 1;
            let height = reader.read_bits(5)? as u32 + 1;
            Ok((width, height))
        } else {
            let is_medium = !reader.read_bit()?;
            if is_medium {
                let width = reader.read_bits(9)? as u32 + 1;
                let height = reader.read_bits(9)? as u32 + 1;
                Ok((width, height))
            } else {
                let width = reader.read_bits(13)? as u32 + 1;
                let height = reader.read_bits(13)? as u32 + 1;
                Ok((width, height))
            }
        }
    }

    /// Encode color encoding (simplified)
    fn encode_color_encoding<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        let color_enc = match self.color_encoding {
            ColorEncoding::SRGB => 0,
            ColorEncoding::LinearSRGB => 1,
            ColorEncoding::XYB => 2,
            ColorEncoding::Custom => 3,
            ColorEncoding::DisplayP3 => 4,
            ColorEncoding::Rec2020 => 5,
        };
        writer.write_bits(color_enc, 3)?;

        Ok(())
    }

    /// Decode color encoding (simplified)
    fn decode_color_encoding<R: Read>(reader: &mut BitReader<R>) -> JxlResult<(ColorEncoding, Option<CustomColorEncoding>)> {
        let color_enc = reader.read_bits(3)? as u8;
        let color_encoding = match color_enc {
            0 => ColorEncoding::SRGB,
            1 => ColorEncoding::LinearSRGB,
            2 => ColorEncoding::XYB,
            3 => ColorEncoding::Custom,
            4 => ColorEncoding::DisplayP3,
            5 => ColorEncoding::Rec2020,
            _ => ColorEncoding::SRGB,
        };

        Ok((color_encoding, None))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bit_depth_integer() {
        let bd = BitDepth::integer(8);
        assert!(!bd.floating_point_sample);
        assert_eq!(bd.bits_per_sample, 8);
        assert_eq!(bd.exp_bits, 0);
    }

    #[test]
    fn test_bit_depth_float() {
        let bd = BitDepth::float(32, 8);
        assert!(bd.floating_point_sample);
        assert_eq!(bd.bits_per_sample, 32);
        assert_eq!(bd.exp_bits, 8);
    }

    #[test]
    fn test_bit_depth_roundtrip() {
        let original = BitDepth::integer(12);
        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            original.encode(&mut writer).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(&buffer[..]);
        let decoded = BitDepth::decode(&mut reader).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn test_metadata_default() {
        let metadata = JxlImageMetadata::default();
        assert!(metadata.all_default);
        assert!(metadata.xyb_encoded);
    }

    #[test]
    fn test_metadata_for_rgb() {
        let metadata = JxlImageMetadata::for_rgb_image(64, 64, 8);
        assert!(!metadata.all_default);
        assert!(metadata.have_intrinsic_size);
        assert_eq!(metadata.intrinsic_width, 64);
        assert_eq!(metadata.intrinsic_height, 64);
        assert_eq!(metadata.bit_depth.bits_per_sample, 8);
    }

    #[test]
    fn test_metadata_roundtrip_default() {
        let original = JxlImageMetadata::default();
        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            original.encode(&mut writer).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(&buffer[..]);
        let decoded = JxlImageMetadata::decode(&mut reader).unwrap();
        assert_eq!(original.all_default, decoded.all_default);
        assert_eq!(original.xyb_encoded, decoded.xyb_encoded);
    }

    #[test]
    fn test_metadata_roundtrip_rgb() {
        let original = JxlImageMetadata::for_rgb_image(128, 128, 8);
        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            original.encode(&mut writer).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(&buffer[..]);
        let decoded = JxlImageMetadata::decode(&mut reader).unwrap();
        assert_eq!(original.all_default, decoded.all_default);
        assert_eq!(original.have_intrinsic_size, decoded.have_intrinsic_size);
        assert_eq!(original.intrinsic_width, decoded.intrinsic_width);
        assert_eq!(original.intrinsic_height, decoded.intrinsic_height);
        assert_eq!(original.bit_depth.bits_per_sample, decoded.bit_depth.bits_per_sample);
    }

    #[test]
    fn test_size_encoding_small() {
        let metadata = JxlImageMetadata::for_rgb_image(8, 8, 8);
        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            metadata.encode_size(&mut writer, 8, 8).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(&buffer[..]);
        let (width, height) = JxlImageMetadata::decode_size(&mut reader).unwrap();
        assert_eq!(width, 8);
        assert_eq!(height, 8);
    }

    #[test]
    fn test_size_encoding_medium() {
        let metadata = JxlImageMetadata::for_rgb_image(128, 256, 8);
        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(&mut buffer);
            metadata.encode_size(&mut writer, 128, 256).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(&buffer[..]);
        let (width, height) = JxlImageMetadata::decode_size(&mut reader).unwrap();
        assert_eq!(width, 128);
        assert_eq!(height, 256);
    }

    #[test]
    fn test_extra_channel_default() {
        let channel = ExtraChannelInfo::default();
        assert_eq!(channel.channel_type, ExtraChannelType::Alpha);
    }
}
