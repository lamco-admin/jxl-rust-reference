//! Color space transformations for JPEG XL
//!
//! This crate implements color space conversions, including:
//! - RGB <-> XYB (JPEG XL's perceptual color space)
//! - sRGB <-> Linear RGB
//! - Color correlation transforms

pub mod correlation;
pub mod srgb;
pub mod xyb;

pub use correlation::*;
pub use srgb::*;
pub use xyb::*;
