//! Animation support for JPEG XL
//!
//! JPEG XL supports animations with:
//! - Multiple frames with individual durations
//! - Frame blending modes
//! - Reference frames for delta encoding
//! - Loop count control

use jxl_bitstream::{BitReader, BitWriter};
use jxl_core::{JxlError, JxlResult};
use std::io::{Read, Write};

/// Animation header information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationHeader {
    /// Time base denominator (ticks per second)
    pub tps_numerator: u32,
    /// Time base numerator
    pub tps_denominator: u32,
    /// Number of loops (0 = infinite)
    pub num_loops: u32,
    /// Whether animation has separate alpha channel timing
    pub have_timecodes: bool,
}

impl Default for AnimationHeader {
    fn default() -> Self {
        Self {
            tps_numerator: 1000, // 1000 ticks per second (1ms resolution)
            tps_denominator: 1,
            num_loops: 0, // Infinite loop by default
            have_timecodes: false,
        }
    }
}

impl AnimationHeader {
    /// Create animation header with specific framerate
    pub fn with_fps(fps: f32) -> Self {
        let tps_numerator = 1000;
        let tps_denominator = 1;

        Self {
            tps_numerator,
            tps_denominator,
            num_loops: 0,
            have_timecodes: false,
        }
    }

    /// Get duration in ticks for a frame with given fps
    pub fn duration_for_fps(&self, fps: f32) -> u32 {
        let seconds_per_frame = 1.0 / fps;
        let ticks_per_second = (self.tps_numerator as f64) / (self.tps_denominator as f64);
        (seconds_per_frame as f64 * ticks_per_second) as u32
    }

    /// Write animation header to bitstream
    pub fn write<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // Write ticks per second (32 bits each)
        writer.write_bits(self.tps_numerator as u64, 32)?;
        writer.write_bits(self.tps_denominator as u64, 32)?;

        // Write loop count (32 bits, 0 = infinite)
        writer.write_bits(self.num_loops as u64, 32)?;

        // Write timecode flag
        writer.write_bit(self.have_timecodes)?;

        Ok(())
    }

    /// Read animation header from bitstream
    pub fn read<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        let tps_numerator = reader.read_bits(32)? as u32;
        let tps_denominator = reader.read_bits(32)? as u32;
        let num_loops = reader.read_bits(32)? as u32;
        let have_timecodes = reader.read_bit()?;

        Ok(Self {
            tps_numerator,
            tps_denominator,
            num_loops,
            have_timecodes,
        })
    }
}

/// Frame blend mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlendMode {
    /// Replace previous frame
    Replace,
    /// Blend with previous frame using alpha
    Blend,
    /// Alpha blend with specific source
    AlphaBlend,
    /// Multiply with previous frame
    Multiply,
}

impl BlendMode {
    /// Encode blend mode to bits
    pub fn to_bits(&self) -> u8 {
        match self {
            BlendMode::Replace => 0,
            BlendMode::Blend => 1,
            BlendMode::AlphaBlend => 2,
            BlendMode::Multiply => 3,
        }
    }

    /// Decode blend mode from bits
    pub fn from_bits(bits: u8) -> JxlResult<Self> {
        match bits {
            0 => Ok(BlendMode::Replace),
            1 => Ok(BlendMode::Blend),
            2 => Ok(BlendMode::AlphaBlend),
            3 => Ok(BlendMode::Multiply),
            _ => Err(JxlError::InvalidBitstream(format!(
                "Invalid blend mode: {}",
                bits
            ))),
        }
    }
}

/// Frame header for animated images
#[derive(Debug, Clone)]
pub struct FrameHeader {
    /// Frame index
    pub frame_index: u32,
    /// Duration in ticks
    pub duration: u32,
    /// Blend mode with previous frame
    pub blend_mode: BlendMode,
    /// Whether this frame is a keyframe (no dependencies)
    pub is_keyframe: bool,
    /// Save frame as reference for future frames
    pub save_as_reference: u8,
    /// Load reference frame for blending
    pub load_reference: u8,
    /// Frame name (optional)
    pub name: Option<String>,
}

impl Default for FrameHeader {
    fn default() -> Self {
        Self {
            frame_index: 0,
            duration: 0,
            blend_mode: BlendMode::Replace,
            is_keyframe: true,
            save_as_reference: 0,
            load_reference: 0,
            name: None,
        }
    }
}

impl FrameHeader {
    /// Create a keyframe
    pub fn keyframe(frame_index: u32, duration: u32) -> Self {
        Self {
            frame_index,
            duration,
            blend_mode: BlendMode::Replace,
            is_keyframe: true,
            save_as_reference: 0,
            load_reference: 0,
            name: None,
        }
    }

    /// Create a delta frame (depends on previous)
    pub fn delta_frame(frame_index: u32, duration: u32, blend_mode: BlendMode) -> Self {
        Self {
            frame_index,
            duration,
            blend_mode,
            is_keyframe: false,
            save_as_reference: 0,
            load_reference: 0,
            name: None,
        }
    }

    /// Write frame header to bitstream
    pub fn write<W: Write>(&self, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // Write frame index (32 bits)
        writer.write_bits(self.frame_index as u64, 32)?;

        // Write duration (32 bits)
        writer.write_bits(self.duration as u64, 32)?;

        // Write blend mode (2 bits)
        writer.write_bits(self.blend_mode.to_bits() as u64, 2)?;

        // Write keyframe flag
        writer.write_bit(self.is_keyframe)?;

        // Write reference frame info
        writer.write_bits(self.save_as_reference as u64, 2)?;
        writer.write_bits(self.load_reference as u64, 2)?;

        // Write frame name if present
        writer.write_bit(self.name.is_some())?;
        if let Some(ref name) = self.name {
            let name_bytes = name.as_bytes();
            writer.write_bits(name_bytes.len() as u64, 16)?;
            for &byte in name_bytes {
                writer.write_bits(byte as u64, 8)?;
            }
        }

        Ok(())
    }

    /// Read frame header from bitstream
    pub fn read<R: Read>(reader: &mut BitReader<R>) -> JxlResult<Self> {
        let frame_index = reader.read_bits(32)? as u32;
        let duration = reader.read_bits(32)? as u32;
        let blend_mode = BlendMode::from_bits(reader.read_bits(2)? as u8)?;
        let is_keyframe = reader.read_bit()?;
        let save_as_reference = reader.read_bits(2)? as u8;
        let load_reference = reader.read_bits(2)? as u8;

        let name = if reader.read_bit()? {
            let name_len = reader.read_bits(16)? as usize;
            let mut name_bytes = vec![0u8; name_len];
            for byte in name_bytes.iter_mut() {
                *byte = reader.read_bits(8)? as u8;
            }
            Some(String::from_utf8(name_bytes).unwrap_or_default())
        } else {
            None
        };

        Ok(Self {
            frame_index,
            duration,
            blend_mode,
            is_keyframe,
            save_as_reference,
            load_reference,
            name,
        })
    }
}

/// Animation sequence manager
#[derive(Debug, Clone)]
pub struct Animation {
    /// Animation header
    pub header: AnimationHeader,
    /// Frames in the animation
    pub frames: Vec<FrameHeader>,
}

impl Animation {
    /// Create a new animation
    pub fn new(header: AnimationHeader) -> Self {
        Self {
            header,
            frames: Vec::new(),
        }
    }

    /// Add a frame to the animation
    pub fn add_frame(&mut self, frame: FrameHeader) {
        self.frames.push(frame);
    }

    /// Get total duration in ticks
    pub fn total_duration(&self) -> u32 {
        self.frames.iter().map(|f| f.duration).sum()
    }

    /// Get duration in seconds
    pub fn duration_seconds(&self) -> f64 {
        let total_ticks = self.total_duration() as f64;
        let tps = (self.header.tps_numerator as f64) / (self.header.tps_denominator as f64);
        total_ticks / tps
    }

    /// Get framerate (if uniform)
    pub fn framerate(&self) -> Option<f32> {
        if self.frames.is_empty() {
            return None;
        }

        // Check if all frames have the same duration
        let first_duration = self.frames[0].duration;
        if self.frames.iter().all(|f| f.duration == first_duration) {
            let tps = (self.header.tps_numerator as f64) / (self.header.tps_denominator as f64);
            let fps = tps / (first_duration as f64);
            Some(fps as f32)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_animation_header_default() {
        let header = AnimationHeader::default();
        assert_eq!(header.tps_numerator, 1000);
        assert_eq!(header.tps_denominator, 1);
        assert_eq!(header.num_loops, 0);
    }

    #[test]
    fn test_animation_header_fps() {
        let header = AnimationHeader::default();
        let duration_30fps = header.duration_for_fps(30.0);
        let duration_60fps = header.duration_for_fps(60.0);

        // At 1000 ticks/sec: 30fps = ~33 ticks, 60fps = ~16 ticks
        assert!(duration_30fps > duration_60fps);
        assert!((duration_30fps as f32 - 33.33).abs() < 1.0);
    }

    #[test]
    fn test_animation_header_roundtrip() {
        let header = AnimationHeader {
            tps_numerator: 1000,
            tps_denominator: 1,
            num_loops: 3,
            have_timecodes: true,
        };

        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(Cursor::new(&mut buffer));
            header.write(&mut writer).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(Cursor::new(&buffer));
        let decoded = AnimationHeader::read(&mut reader).unwrap();

        assert_eq!(header, decoded);
    }

    #[test]
    fn test_blend_mode_roundtrip() {
        for mode in &[
            BlendMode::Replace,
            BlendMode::Blend,
            BlendMode::AlphaBlend,
            BlendMode::Multiply,
        ] {
            let bits = mode.to_bits();
            let decoded = BlendMode::from_bits(bits).unwrap();
            assert_eq!(*mode, decoded);
        }
    }

    #[test]
    fn test_frame_header_roundtrip() {
        let frame = FrameHeader {
            frame_index: 5,
            duration: 100,
            blend_mode: BlendMode::Blend,
            is_keyframe: false,
            save_as_reference: 1,
            load_reference: 0,
            name: Some("test_frame".to_string()),
        };

        let mut buffer = Vec::new();
        {
            let mut writer = BitWriter::new(Cursor::new(&mut buffer));
            frame.write(&mut writer).unwrap();
            writer.flush().unwrap();
        }

        let mut reader = BitReader::new(Cursor::new(&buffer));
        let decoded = FrameHeader::read(&mut reader).unwrap();

        assert_eq!(frame.frame_index, decoded.frame_index);
        assert_eq!(frame.duration, decoded.duration);
        assert_eq!(frame.blend_mode, decoded.blend_mode);
        assert_eq!(frame.is_keyframe, decoded.is_keyframe);
        assert_eq!(frame.name, decoded.name);
    }

    #[test]
    fn test_animation_duration() {
        let header = AnimationHeader::default();
        let mut animation = Animation::new(header);

        animation.add_frame(FrameHeader::keyframe(0, 100));
        animation.add_frame(FrameHeader::keyframe(1, 200));
        animation.add_frame(FrameHeader::keyframe(2, 150));

        assert_eq!(animation.total_duration(), 450);
        assert!((animation.duration_seconds() - 0.45).abs() < 0.001);
    }

    #[test]
    fn test_animation_framerate() {
        let header = AnimationHeader::default();
        let mut animation = Animation::new(header);

        // Add frames with uniform duration (33 ticks = ~30fps)
        for i in 0..10 {
            animation.add_frame(FrameHeader::keyframe(i, 33));
        }

        let fps = animation.framerate().unwrap();
        assert!((fps - 30.30).abs() < 0.5); // ~30fps
    }
}
