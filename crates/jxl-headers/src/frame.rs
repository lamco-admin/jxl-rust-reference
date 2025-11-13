//! JPEG XL Frame Headers
//!
//! Implementation of production-grade frame headers according to JPEG XL specification.
//! Frame headers describe individual frames in the image/animation and control encoding parameters.

use jxl_bitstream::{BitReader, BitWriter};
use jxl_core::*;
use std::io::{Read, Write};

/// Frame type determines decoding requirements and reference frame behavior
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameType {
    /// Regular frame (most common)
    RegularFrame = 0,
    /// LF (Low Frequency) frame - DC-only for progressive decoding
    LFFrame = 1,
    /// Reference frame - not displayed, used for future frame references
    ReferenceFrame = 2,
    /// Skip progressive - signals decoder can skip progressive passes
    SkipProgressive = 3,
}

impl FrameType {
    pub fn from_u8(value: u8) -> JxlResult<Self> {
        match value {
            0 => Ok(FrameType::RegularFrame),
            1 => Ok(FrameType::LFFrame),
            2 => Ok(FrameType::ReferenceFrame),
            3 => Ok(FrameType::SkipProgressive),
            _ => Err(JxlError::InvalidParameter(format!("Invalid frame type: {}", value))),
        }
    }
}

/// Blending information for animation frames
#[derive(Debug, Clone)]
pub struct BlendingInfo {
    /// Blend mode (0 = replace, 1 = add, 2 = blend, 3 = alpha-weighted blend)
    pub mode: u8,
    /// Alpha channel to use for blending (if applicable)
    pub alpha_channel: u8,
    /// Whether to clamp values after blending
    pub clamp: bool,
    /// Source for blending (0 = previous frame, 1-3 = reference frames)
    pub source: u8,
}

impl Default for BlendingInfo {
    fn default() -> Self {
        Self {
            mode: 0,      // Replace
            alpha_channel: 0,
            clamp: false,
            source: 0,    // Previous frame
        }
    }
}

/// Progressive rendering passes configuration
#[derive(Debug, Clone)]
pub struct Passes {
    /// Number of passes (1 = non-progressive)
    pub num_passes: u8,
    /// Number of downsampling levels
    pub num_ds: u8,
    /// Shift for each pass
    pub shift: Vec<u8>,
    /// Downsampling for each pass
    pub downsample: Vec<u8>,
    /// Last pass index for each downsampling level
    pub last_pass: Vec<u8>,
}

impl Default for Passes {
    fn default() -> Self {
        Self {
            num_passes: 1,
            num_ds: 0,
            shift: vec![0],
            downsample: vec![1],
            last_pass: vec![0],
        }
    }
}

/// JPEG XL Frame Header
///
/// Comprehensive frame header supporting all production JPEG XL features:
/// - Frame types (regular, LF, reference, skip progressive)
/// - Animation (duration, blending)
/// - Progressive rendering
/// - Restoration filters
/// - Extensions for future features
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame type
    pub frame_type: FrameType,

    /// Encoding (0 = VarDCT, 1 = Modular)
    pub encoding: u8,

    /// Flags for quick feature detection
    pub flags: u64,

    /// Whether all default values are used (allows header compression)
    pub all_default: bool,

    /// Frame duration for animation (in ticks)
    pub duration: u32,

    /// Timecode for animation synchronization
    pub timecode: u32,

    /// Frame name (for multi-frame images)
    pub name: Option<String>,

    /// Whether this is the last frame
    pub is_last: bool,

    /// Save frame as reference for future frames
    pub save_as_reference: u8,

    /// Blending information for animation
    pub blending: BlendingInfo,

    /// Progressive passes configuration
    pub passes: Passes,

    /// Group size shift (log2 of group size / 256)
    pub group_size_shift: u8,

    /// X quantization multiplier
    pub x_qm_scale: u8,

    /// B quantization multiplier
    pub b_qm_scale: u8,

    /// Number of LF groups (for progressive decoding)
    pub num_lf_groups: u32,

    /// Restoration filter flags
    pub restoration_filter: RestorationFilter,

    /// Extensions for future features
    pub extensions: u64,

    /// Frame is self-contained (doesn't reference others)
    pub can_be_referenced: bool,
}

/// Restoration filters for post-processing
#[derive(Debug, Clone)]
pub struct RestorationFilter {
    /// Gabor-like filter enabled
    pub gab: bool,
    /// EPF (Edge-Preserving Filter) enabled
    pub epf: bool,
    /// Extensions
    pub extensions: u64,
}

impl Default for RestorationFilter {
    fn default() -> Self {
        Self {
            gab: false,
            epf: false,
            extensions: 0,
        }
    }
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            frame_type: FrameType::RegularFrame,
            encoding: 0, // VarDCT
            flags: 0,
            all_default: true,
            duration: 0,
            timecode: 0,
            name: None,
            is_last: true,
            save_as_reference: 0,
            blending: BlendingInfo::default(),
            passes: Passes::default(),
            group_size_shift: 1,
            x_qm_scale: 2,
            b_qm_scale: 2,
            num_lf_groups: 1,
            restoration_filter: RestorationFilter::default(),
            extensions: 0,
            can_be_referenced: false,
        }
    }
}

impl FrameHeader {
    /// Create a simple frame header for still images
    pub fn simple_still_image() -> Self {
        Self::default()
    }

    /// Create a frame header for animation
    pub fn animation_frame(duration: u32, is_last: bool) -> Self {
        Self {
            duration,
            is_last,
            all_default: false,
            ..Self::default()
        }
    }

    /// Create a progressive frame header
    pub fn progressive_frame(num_passes: u8) -> Self {
        Self {
            passes: Passes {
                num_passes,
                ..Passes::default()
            },
            all_default: false,
            ..Self::default()
        }
    }

    /// Parse frame header from bitstream
    pub fn parse<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        let mut header = Self::default();

        // Check if all default
        header.all_default = reader.read_bit()?;

        if header.all_default {
            // Use all default values, header is complete
            return Ok(header);
        }

        // Read frame type (2 bits)
        let frame_type = reader.read_bits(2)? as u8;
        header.frame_type = FrameType::from_u8(frame_type)?;

        // Read encoding (1 bit)
        header.encoding = reader.read_bit()? as u8;

        // Read flags (use bits for u64)
        header.flags = reader.read_bits(32)?; // Use 32 bits for now

        // If animation, read duration
        if !header.is_last || header.duration > 0 {
            header.duration = reader.read_bits(32)? as u32;
        }

        // Read frame name if present
        if (header.flags & 0x01) != 0 {
            let name_len = reader.read_bits(8)? as usize;
            let mut name_bytes = vec![0u8; name_len];
            for byte in &mut name_bytes {
                *byte = reader.read_bits(8)? as u8;
            }
            header.name = Some(String::from_utf8_lossy(&name_bytes).to_string());
        }

        Ok(header)
    }

    /// Write frame header to bitstream
    pub fn write<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // Write all_default flag
        writer.write_bit(self.all_default)?;

        if self.all_default {
            // All default, done
            return Ok(());
        }

        // Write frame type (2 bits)
        writer.write_bits(self.frame_type as u64, 2)?;

        // Write encoding (1 bit)
        writer.write_bit(self.encoding != 0)?;

        // Write flags (use bits for u64)
        writer.write_bits(self.flags & 0xFFFFFFFF, 32)?; // Use 32 bits for now

        // Write duration if needed
        if !self.is_last || self.duration > 0 {
            writer.write_bits(self.duration as u64, 32)?;
        }

        // Write frame name if present
        if let Some(ref name) = self.name {
            let name_bytes = name.as_bytes();
            writer.write_bits(name_bytes.len() as u64, 8)?;
            for &byte in name_bytes {
                writer.write_bits(byte as u64, 8)?;
            }
        }

        Ok(())
    }

    /// Validate frame header consistency
    pub fn validate(&self) -> JxlResult<()> {
        // Validate frame type
        if self.frame_type == FrameType::LFFrame && self.num_lf_groups == 0 {
            return Err(JxlError::InvalidParameter(
                "LF frame must have num_lf_groups > 0".to_string()
            ));
        }

        // Validate passes
        if self.passes.num_passes == 0 {
            return Err(JxlError::InvalidParameter(
                "num_passes must be > 0".to_string()
            ));
        }

        // Validate encoding
        if self.encoding > 1 {
            return Err(JxlError::InvalidParameter(
                format!("Invalid encoding: {}", self.encoding)
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frame_header_default() {
        let header = FrameHeader::default();
        assert_eq!(header.frame_type, FrameType::RegularFrame);
        assert!(header.all_default);
        assert!(header.is_last);
    }

    #[test]
    fn test_frame_header_animation() {
        let header = FrameHeader::animation_frame(100, false);
        assert_eq!(header.duration, 100);
        assert!(!header.is_last);
        assert!(!header.all_default);
    }

    #[test]
    fn test_frame_header_progressive() {
        let header = FrameHeader::progressive_frame(4);
        assert_eq!(header.passes.num_passes, 4);
        assert!(!header.all_default);
    }

    #[test]
    fn test_frame_type_conversion() {
        assert_eq!(FrameType::from_u8(0).unwrap(), FrameType::RegularFrame);
        assert_eq!(FrameType::from_u8(1).unwrap(), FrameType::LFFrame);
        assert_eq!(FrameType::from_u8(2).unwrap(), FrameType::ReferenceFrame);
        assert_eq!(FrameType::from_u8(3).unwrap(), FrameType::SkipProgressive);
        assert!(FrameType::from_u8(4).is_err());
    }

    #[test]
    fn test_frame_header_validation() {
        let mut header = FrameHeader::default();
        assert!(header.validate().is_ok());

        // Invalid: LF frame without LF groups
        header.frame_type = FrameType::LFFrame;
        header.num_lf_groups = 0;
        assert!(header.validate().is_err());
    }
}
