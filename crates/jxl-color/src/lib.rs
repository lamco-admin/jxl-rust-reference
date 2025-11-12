//! Color space transformations for JPEG XL
//!
//! This crate implements color space conversions, including:
//! - RGB <-> XYB (JPEG XL's perceptual color space)
//! - sRGB <-> Linear RGB
//! - Color correlation transforms

pub mod xyb;
pub mod srgb;
pub mod correlation;

pub use xyb::*;
pub use srgb::*;
pub use correlation::*;
