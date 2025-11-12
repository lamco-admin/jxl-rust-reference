//! # JPEG XL - Rust Reference Implementation
//!
//! This crate provides a high-level API for encoding and decoding JPEG XL images.
//!
//! ## Quick Start
//!
//! ### Decoding
//!
//! ```no_run
//! use jxl::JxlDecoder;
//!
//! let mut decoder = JxlDecoder::new();
//! let image = decoder.decode_file("input.jxl").unwrap();
//! println!("Decoded {}x{} image", image.width(), image.height());
//! ```
//!
//! ### Encoding
//!
//! ```no_run
//! use jxl::{JxlEncoder, EncoderOptions, Image, Dimensions, ColorChannels, PixelType, ColorEncoding};
//!
//! let dimensions = Dimensions::new(800, 600);
//! let image = Image::new(
//!     dimensions,
//!     ColorChannels::RGB,
//!     PixelType::U8,
//!     ColorEncoding::SRGB,
//! ).unwrap();
//!
//! let options = EncoderOptions::default()
//!     .quality(90.0)
//!     .effort(7);
//!
//! let encoder = JxlEncoder::new(options);
//! encoder.encode_file(&image, "output.jxl").unwrap();
//! ```
//!
//! ## Features
//!
//! - Full JPEG XL encoding and decoding
//! - Support for multiple bit depths (8-bit, 16-bit, float)
//! - Lossless and lossy compression
//! - XYB color space support
//! - Multi-threaded processing
//! - ANS entropy coding
//!
//! ## Architecture
//!
//! This implementation is based on the official libjxl C++ reference implementation
//! and follows the ISO/IEC 18181 standard.

// Re-export core types
pub use jxl_core::{
    ColorChannels, ColorEncoding, Dimensions, Image, ImageBuffer, JxlError, JxlResult,
    Orientation, PixelType, Sample,
};

// Re-export decoder
pub use jxl_decoder::JxlDecoder;

// Re-export encoder
pub use jxl_encoder::{EncoderOptions, JxlEncoder};

/// Library version
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// JPEG XL specification version this implementation targets
pub const SPEC_VERSION: &str = "ISO/IEC 18181:2022";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }

    #[test]
    fn test_image_creation() {
        let dims = Dimensions::new(100, 100);
        let image = Image::new(
            dims,
            ColorChannels::RGB,
            PixelType::U8,
            ColorEncoding::SRGB,
        );
        assert!(image.is_ok());
        let img = image.unwrap();
        assert_eq!(img.width(), 100);
        assert_eq!(img.height(), 100);
    }
}
