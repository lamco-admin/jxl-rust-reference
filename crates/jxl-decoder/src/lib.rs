//! JPEG XL decoder implementation

use jxl_bitstream::BitReader;
use jxl_color::{linear_f32_to_srgb_u8, xyb_to_rgb};
use jxl_core::*;
use jxl_headers::JxlHeader;
use jxl_transform::{
    dequantize, generate_xyb_quant_tables, idct_channel, inv_zigzag_scan_channel, merge_dc_ac,
    BLOCK_SIZE,
};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

/// JPEG XL decoder
pub struct JxlDecoder {
    header: Option<JxlHeader>,
}

impl JxlDecoder {
    pub fn new() -> Self {
        Self { header: None }
    }

    /// Decode a JPEG XL file from a path
    pub fn decode_file<P: AsRef<Path>>(&mut self, path: P) -> JxlResult<Image> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        self.decode(reader)
    }

    /// Decode from a reader
    pub fn decode<R: Read>(&mut self, reader: R) -> JxlResult<Image> {
        let mut bit_reader = BitReader::new(reader);

        // Parse header
        let header = JxlHeader::parse(&mut bit_reader)?;
        self.header = Some(header.clone());

        // Determine pixel type based on bit depth
        let pixel_type = if header.bit_depth <= 8 {
            PixelType::U8
        } else if header.bit_depth <= 16 {
            PixelType::U16
        } else {
            PixelType::F32
        };

        // Determine channels
        let channels = match header.num_channels {
            1 => ColorChannels::Gray,
            2 => ColorChannels::GrayAlpha,
            3 => ColorChannels::RGB,
            4 => ColorChannels::RGBA,
            _ => {
                return Err(JxlError::UnsupportedFeature(format!(
                    "{} channels not supported",
                    header.num_channels
                )))
            }
        };

        // Create image buffer
        let mut image = Image::new(
            header.dimensions,
            channels,
            pixel_type,
            header.color_encoding,
        )?;

        // Decode frame data
        self.decode_frame(&mut bit_reader, &mut image)?;

        Ok(image)
    }

    fn decode_frame<R: Read>(&self, reader: &mut BitReader<R>, image: &mut Image) -> JxlResult<()> {
        let header = self.header.as_ref().unwrap();

        // Full decoding pipeline:
        // 1. Decode quantized coefficients from bitstream
        // 2. Dequantize coefficients
        // 3. Apply inverse DCT
        // 4. Convert XYB to RGB color space
        // 5. Convert linear RGB to sRGB
        // 6. Convert to target pixel format

        let width = header.dimensions.width as usize;
        let height = header.dimensions.height as usize;
        let num_channels = header.num_channels;

        // Only support RGB/RGBA for now
        if num_channels < 3 {
            return Err(JxlError::UnsupportedFeature(
                "Only RGB/RGBA images are currently supported".to_string(),
            ));
        }

        // Step 1: Decode quantized coefficients
        let quantized = self.decode_coefficients(reader, width, height)?;

        // Step 2: Dequantize with XYB-tuned tables
        // Use per-channel dequantization matching encoder
        let xyb_tables = generate_xyb_quant_tables(consts::DEFAULT_QUALITY);
        let mut dct_coeffs = vec![vec![0.0; width * height]; 3];

        // X channel (index 0)
        self.dequantize_channel(&quantized[0], &xyb_tables.x_table, width, height, &mut dct_coeffs[0]);

        // Y channel (index 1)
        self.dequantize_channel(&quantized[1], &xyb_tables.y_table, width, height, &mut dct_coeffs[1]);

        // B-Y channel (index 2)
        self.dequantize_channel(&quantized[2], &xyb_tables.b_table, width, height, &mut dct_coeffs[2]);

        // Step 3: Apply inverse DCT
        let mut xyb = vec![vec![0.0; width * height]; 3];
        for c in 0..3 {
            idct_channel(&dct_coeffs[c], width, height, &mut xyb[c]);
        }

        // Step 4: Convert XYB to RGB
        let mut linear_rgb = vec![0.0; width * height * 3];
        self.xyb_to_rgb_image(&xyb, &mut linear_rgb, width, height);

        // Step 5: Decode alpha channel if present
        let linear_rgba = if num_channels == 4 {
            let mut rgba = vec![0.0; width * height * 4];
            for i in 0..(width * height) {
                rgba[i * 4] = linear_rgb[i * 3];
                rgba[i * 4 + 1] = linear_rgb[i * 3 + 1];
                rgba[i * 4 + 2] = linear_rgb[i * 3 + 2];
            }
            self.decode_alpha_channel(reader, &mut rgba, width, height)?;
            rgba
        } else {
            linear_rgb
        };

        // Step 6: Convert to target pixel format
        self.convert_to_target_format(&linear_rgba, image, width, height, num_channels)?;

        Ok(())
    }

    /// Decode quantized DCT coefficients with DC/AC separation
    fn decode_coefficients<R: Read>(
        &self,
        reader: &mut BitReader<R>,
        width: usize,
        height: usize,
    ) -> JxlResult<Vec<Vec<i16>>> {
        let mut quantized = vec![vec![0i16; width * height]; 3];

        // Calculate number of blocks for AC array sizing
        let blocks_x = width.div_ceil(8);
        let blocks_y = height.div_ceil(8);
        let num_blocks = blocks_x * blocks_y;

        for channel_data in quantized.iter_mut().take(3) {
            // Decode DC and AC coefficients separately
            let dc_coeffs = self.decode_dc_coefficients(reader)?;
            let ac_coeffs = self.decode_ac_coefficients(reader, num_blocks)?;

            // Merge DC and AC back into zigzag format
            let mut zigzag_data = Vec::new();
            merge_dc_ac(&dc_coeffs, &ac_coeffs, &mut zigzag_data);

            // Apply inverse zigzag to restore spatial block order
            let mut spatial_data = Vec::new();
            inv_zigzag_scan_channel(&zigzag_data, width, height, &mut spatial_data);

            // Copy to output (may be smaller than spatial_data due to padding)
            for (i, &val) in spatial_data.iter().enumerate().take(width * height) {
                channel_data[i] = val;
            }
        }

        Ok(quantized)
    }

    /// Decode DC coefficients using differential decoding
    fn decode_dc_coefficients<R: Read>(
        &self,
        reader: &mut BitReader<R>,
    ) -> JxlResult<Vec<i16>> {
        // Read number of DC coefficients
        let num_dc = reader.read_u32(20)? as usize;

        if num_dc == 0 {
            return Ok(Vec::new());
        }

        let mut dc_coeffs = Vec::with_capacity(num_dc);

        // Decode first DC value directly
        let first_dc = self.decode_coefficient_value(reader)?;
        dc_coeffs.push(first_dc);

        // Decode remaining DC values as differences (differential decoding)
        let mut prev_dc = first_dc;
        for _ in 1..num_dc {
            let diff = self.decode_coefficient_value(reader)?;
            let dc = prev_dc + diff;
            dc_coeffs.push(dc);
            prev_dc = dc;
        }

        Ok(dc_coeffs)
    }

    /// Decode AC coefficients from sparse encoding
    fn decode_ac_coefficients<R: Read>(
        &self,
        reader: &mut BitReader<R>,
        num_blocks: usize,
    ) -> JxlResult<Vec<i16>> {
        // Read number of non-zero AC coefficients
        let non_zero_count = reader.read_u32(20)? as usize;

        // AC array size: 63 coefficients per block (64 total - 1 DC)
        let ac_size = num_blocks * 63;
        let mut ac_coeffs = vec![0i16; ac_size];

        // Decode non-zero AC coefficients
        for _ in 0..non_zero_count {
            let pos = reader.read_u32(20)? as usize;
            if pos < ac_coeffs.len() {
                ac_coeffs[pos] = self.decode_coefficient_value(reader)?;
            }
        }

        Ok(ac_coeffs)
    }

    /// Decode a single coefficient value
    fn decode_coefficient_value<R: Read>(
        &self,
        reader: &mut BitReader<R>,
    ) -> JxlResult<i16> {
        // Read sign
        let is_negative = reader.read_bit()?;

        // Read absolute value
        let bits_needed = reader.read_bits(4)? as usize;
        let abs_val = if bits_needed > 0 {
            reader.read_bits(bits_needed)? as i16
        } else {
            0
        };

        Ok(if is_negative { -abs_val } else { abs_val })
    }

    /// Dequantize a channel of DCT coefficients
    fn dequantize_channel(
        &self,
        quantized: &[i16],
        quant_table: &[u16; 64],
        width: usize,
        height: usize,
        output: &mut [f32],
    ) {
        let mut block = [0i16; 64];
        let mut dequant_block = [0.0f32; 64];

        for block_y in (0..height).step_by(BLOCK_SIZE) {
            for block_x in (0..width).step_by(BLOCK_SIZE) {
                // Extract block
                for y in 0..BLOCK_SIZE.min(height - block_y) {
                    for x in 0..BLOCK_SIZE.min(width - block_x) {
                        block[y * BLOCK_SIZE + x] =
                            quantized[(block_y + y) * width + (block_x + x)];
                    }
                }

                // Dequantize
                dequantize(&block, quant_table, &mut dequant_block);

                // Store
                for y in 0..BLOCK_SIZE.min(height - block_y) {
                    for x in 0..BLOCK_SIZE.min(width - block_x) {
                        output[(block_y + y) * width + (block_x + x)] =
                            dequant_block[y * BLOCK_SIZE + x];
                    }
                }
            }
        }
    }

    /// Convert XYB to RGB for entire image
    fn xyb_to_rgb_image(&self, xyb: &[Vec<f32>], rgb: &mut [f32], width: usize, height: usize) {
        let pixel_count = width * height;

        for i in 0..pixel_count {
            let x = xyb[0][i];
            let y = xyb[1][i];
            let b_minus_y = xyb[2][i];

            let (r, g, b) = xyb_to_rgb(x, y, b_minus_y);

            rgb[i * 3] = r.clamp(0.0, 1.0);
            rgb[i * 3 + 1] = g.clamp(0.0, 1.0);
            rgb[i * 3 + 2] = b.clamp(0.0, 1.0);
        }
    }

    /// Decode alpha channel
    fn decode_alpha_channel<R: Read>(
        &self,
        reader: &mut BitReader<R>,
        rgba: &mut [f32],
        width: usize,
        height: usize,
    ) -> JxlResult<()> {
        for i in 0..(width * height) {
            let alpha_u8 = reader.read_bits(8)? as u8;
            rgba[i * 4 + 3] = alpha_u8 as f32 / 255.0;
        }

        Ok(())
    }

    /// Convert linear RGB/RGBA to target pixel format
    fn convert_to_target_format(
        &self,
        linear: &[f32],
        image: &mut Image,
        width: usize,
        height: usize,
        num_channels: usize,
    ) -> JxlResult<()> {
        match &mut image.buffer {
            ImageBuffer::U8(ref mut buffer) => {
                // Convert linear to sRGB U8
                for i in 0..(width * height * num_channels) {
                    buffer[i] = linear_f32_to_srgb_u8(linear[i]);
                }
            }
            ImageBuffer::U16(ref mut buffer) => {
                // Convert linear to U16
                for i in 0..(width * height * num_channels) {
                    let srgb = jxl_color::linear_to_srgb(linear[i]);
                    buffer[i] = (srgb * 65535.0).round().clamp(0.0, 65535.0) as u16;
                }
            }
            ImageBuffer::F32(ref mut buffer) => {
                // Convert linear to sRGB F32
                for i in 0..(width * height * num_channels) {
                    buffer[i] = jxl_color::linear_to_srgb(linear[i]);
                }
            }
        }

        Ok(())
    }

    /// Get the decoded header
    pub fn header(&self) -> Option<&JxlHeader> {
        self.header.as_ref()
    }
}

impl Default for JxlDecoder {
    fn default() -> Self {
        Self::new()
    }
}
