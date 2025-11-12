//! Core types and utilities for JPEG XL implementation
//!
//! This crate provides the fundamental data structures and types used throughout
//! the JPEG XL implementation, including image metadata, pixel formats, and error types.

pub mod consts;
pub mod error;
pub mod image;
pub mod metadata;
pub mod types;

pub use error::{JxlError, JxlResult};
pub use image::*;
pub use metadata::*;
pub use types::*;

/// JPEG XL file signature
pub const JXL_SIGNATURE: [u8; 12] = [
    0xFF, 0x0A, // JXL codestream signature
    0x00, 0x00, 0x00, 0x0C, // Box size
    0x4A, 0x58, 0x4C, 0x20, // 'JXL '
    0x0D, 0x0A, // CR LF
];

/// Minimum version for JPEG XL format
pub const JXL_VERSION: u32 = 0;
