//! Image data structures

use crate::{ColorChannels, ColorEncoding, Dimensions, JxlError, JxlResult, PixelType, Sample};

/// Image buffer that can hold different pixel types
#[derive(Debug, Clone)]
pub enum ImageBuffer {
    U8(Vec<u8>),
    U16(Vec<u16>),
    F32(Vec<f32>),
}

impl ImageBuffer {
    pub fn new(pixel_type: PixelType, size: usize) -> Self {
        match pixel_type {
            PixelType::U8 => ImageBuffer::U8(vec![0; size]),
            PixelType::U16 | PixelType::F16 => ImageBuffer::U16(vec![0; size]),
            PixelType::F32 => ImageBuffer::F32(vec![0.0; size]),
        }
    }

    pub fn len(&self) -> usize {
        match self {
            ImageBuffer::U8(v) => v.len(),
            ImageBuffer::U16(v) => v.len(),
            ImageBuffer::F32(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A decoded or to-be-encoded image
#[derive(Debug, Clone)]
pub struct Image {
    pub dimensions: Dimensions,
    pub channels: ColorChannels,
    pub pixel_type: PixelType,
    pub color_encoding: ColorEncoding,
    pub buffer: ImageBuffer,
}

impl Image {
    pub fn new(
        dimensions: Dimensions,
        channels: ColorChannels,
        pixel_type: PixelType,
        color_encoding: ColorEncoding,
    ) -> JxlResult<Self> {
        if dimensions.width == 0 || dimensions.height == 0 {
            return Err(JxlError::InvalidDimensions {
                width: dimensions.width,
                height: dimensions.height,
            });
        }

        let pixel_count = dimensions.pixel_count();
        let buffer_size = pixel_count * channels.count();
        let buffer = ImageBuffer::new(pixel_type, buffer_size);

        Ok(Self {
            dimensions,
            channels,
            pixel_type,
            color_encoding,
            buffer,
        })
    }

    pub fn width(&self) -> u32 {
        self.dimensions.width
    }

    pub fn height(&self) -> u32 {
        self.dimensions.height
    }

    pub fn pixel_count(&self) -> usize {
        self.dimensions.pixel_count()
    }

    pub fn channel_count(&self) -> usize {
        self.channels.count()
    }
}

/// Frame information for animated images
#[derive(Debug, Clone)]
pub struct Frame {
    pub image: Image,
    pub duration_ms: u32,
    pub name: Option<String>,
}
