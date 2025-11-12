//! Core types for JPEG XL

use num_traits::NumCast;

/// Pixel data type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelType {
    /// 8-bit unsigned integer
    U8,
    /// 16-bit unsigned integer
    U16,
    /// 16-bit floating point
    F16,
    /// 32-bit floating point
    F32,
}

impl PixelType {
    /// Returns the size in bytes for this pixel type
    pub fn bytes_per_pixel(&self) -> usize {
        match self {
            PixelType::U8 => 1,
            PixelType::U16 => 2,
            PixelType::F16 => 2,
            PixelType::F32 => 4,
        }
    }
}

/// Color encoding information
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorEncoding {
    /// sRGB color space
    SRGB,
    /// Linear sRGB
    LinearSRGB,
    /// Display P3
    DisplayP3,
    /// Rec. 2020
    Rec2020,
    /// XYB color space (JPEG XL internal)
    XYB,
    /// Custom color space
    Custom,
}

/// Number of color channels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorChannels {
    /// Grayscale
    Gray = 1,
    /// Grayscale + Alpha
    GrayAlpha = 2,
    /// RGB
    RGB = 3,
    /// RGBA
    RGBA = 4,
}

impl ColorChannels {
    pub fn count(&self) -> usize {
        *self as usize
    }

    pub fn has_alpha(&self) -> bool {
        matches!(self, ColorChannels::GrayAlpha | ColorChannels::RGBA)
    }
}

/// Image dimensions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    pub fn pixel_count(&self) -> usize {
        (self.width as usize) * (self.height as usize)
    }
}

/// Orientation of the image
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Identity = 1,
    FlipHorizontal = 2,
    Rotate180 = 3,
    FlipVertical = 4,
    Transpose = 5,
    Rotate90 = 6,
    AntiTranspose = 7,
    Rotate270 = 8,
}

/// Image sample type
pub trait Sample: Copy + NumCast + PartialOrd {
    const PIXEL_TYPE: PixelType;

    fn to_f32(self) -> f32;
    fn from_f32(value: f32) -> Self;
}

impl Sample for u8 {
    const PIXEL_TYPE: PixelType = PixelType::U8;

    fn to_f32(self) -> f32 {
        self as f32 / 255.0
    }

    fn from_f32(value: f32) -> Self {
        (value * 255.0).round() as u8
    }
}

impl Sample for u16 {
    const PIXEL_TYPE: PixelType = PixelType::U16;

    fn to_f32(self) -> f32 {
        self as f32 / 65535.0
    }

    fn from_f32(value: f32) -> Self {
        (value * 65535.0).round() as u16
    }
}

impl Sample for f32 {
    const PIXEL_TYPE: PixelType = PixelType::F32;

    fn to_f32(self) -> f32 {
        self
    }

    fn from_f32(value: f32) -> Self {
        value
    }
}
