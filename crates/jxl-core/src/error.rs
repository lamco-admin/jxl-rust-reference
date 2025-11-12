//! Error types for JPEG XL operations

use thiserror::Error;

/// Result type for JPEG XL operations
pub type JxlResult<T> = Result<T, JxlError>;

/// Errors that can occur during JPEG XL encoding/decoding
#[derive(Error, Debug)]
pub enum JxlError {
    #[error("Invalid file signature")]
    InvalidSignature,

    #[error("Unsupported version: {0}")]
    UnsupportedVersion(u32),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Invalid bitstream: {0}")]
    InvalidBitstream(String),

    #[error("Decoding error: {0}")]
    DecodingError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),

    #[error("Out of memory")]
    OutOfMemory,

    #[error("Invalid dimensions: {width}x{height}")]
    InvalidDimensions { width: u32, height: u32 },

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Buffer too small: expected {expected}, got {actual}")]
    BufferTooSmall { expected: usize, actual: usize },
}
