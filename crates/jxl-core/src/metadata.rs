//! Image metadata structures

use crate::{ColorEncoding, Dimensions, Orientation};

/// EXIF metadata
#[derive(Debug, Clone, Default)]
pub struct ExifData {
    pub data: Vec<u8>,
}

/// XMP metadata
#[derive(Debug, Clone, Default)]
pub struct XmpData {
    pub data: Vec<u8>,
}

/// ICC color profile
#[derive(Debug, Clone, Default)]
pub struct IccProfile {
    pub data: Vec<u8>,
}

/// Animation metadata
#[derive(Debug, Clone)]
pub struct AnimationMetadata {
    pub num_loops: u32,
    pub have_timecodes: bool,
}

/// Complete image metadata
#[derive(Debug, Clone)]
pub struct ImageMetadata {
    pub dimensions: Dimensions,
    pub color_encoding: ColorEncoding,
    pub orientation: Orientation,
    pub bits_per_sample: u8,
    pub exif: Option<ExifData>,
    pub xmp: Option<XmpData>,
    pub icc_profile: Option<IccProfile>,
    pub animation: Option<AnimationMetadata>,
}

impl Default for ImageMetadata {
    fn default() -> Self {
        Self {
            dimensions: Dimensions::new(0, 0),
            color_encoding: ColorEncoding::SRGB,
            orientation: Orientation::Identity,
            bits_per_sample: 8,
            exif: None,
            xmp: None,
            icc_profile: None,
            animation: None,
        }
    }
}
