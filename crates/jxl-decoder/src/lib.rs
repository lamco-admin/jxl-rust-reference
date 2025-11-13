//! JPEG XL decoder implementation

use jxl_bitstream::{ans::*, BitReader};
use jxl_color::{linear_f32_to_srgb_u8, xyb_to_rgb_image_simd};
use jxl_core::*;
use jxl_headers::{Container, JxlHeader};
use jxl_transform::{
    dequantize, generate_xyb_quant_tables, idct_channel_simd,
    inv_zigzag_scan_channel, merge_dc_ac, BLOCK_SIZE,
};
use rayon::prelude::*;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
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

    /// Decode from a reader (supports both container and naked codestream)
    pub fn decode<R: Read>(&mut self, mut reader: R) -> JxlResult<Image> {
        // Step 1: Read input into buffer to support container detection
        let mut input_data = Vec::new();
        reader.read_to_end(&mut input_data)?;

        // Step 2: Try to parse as container format first
        let codestream = if input_data.starts_with(&jxl_headers::CONTAINER_SIGNATURE) {
            // Parse as container and extract codestream
            let container = Container::read(&mut Cursor::new(&input_data))?;
            container.extract_codestream()?
        } else {
            // Use data directly as naked codestream
            input_data
        };

        // Step 3: Parse header from codestream
        let mut bit_reader = BitReader::new(Cursor::new(&codestream));

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

        // Step 2: Dequantize with XYB-tuned tables (parallel)
        // Use per-channel dequantization matching encoder
        let xyb_tables = generate_xyb_quant_tables(consts::DEFAULT_QUALITY);
        let quant_tables = [&xyb_tables.x_table, &xyb_tables.y_table, &xyb_tables.b_table];

        let dct_coeffs: Vec<Vec<f32>> = quantized
            .par_iter()
            .zip(quant_tables.par_iter())
            .map(|(quantized_channel, quant_table)| {
                let mut dct_coeff = vec![0.0; width * height];
                self.dequantize_channel(quantized_channel, quant_table, width, height, &mut dct_coeff);
                dct_coeff
            })
            .collect();

        // Step 3: Apply inverse DCT (parallel with SIMD)
        let xyb: Vec<Vec<f32>> = dct_coeffs
            .par_iter()
            .map(|dct_coeff| {
                let mut xyb_channel = vec![0.0; width * height];
                idct_channel_simd(dct_coeff, width, height, &mut xyb_channel);
                xyb_channel
            })
            .collect();

        // Step 4: Convert XYB to RGB (SIMD-optimized)
        let mut linear_rgb = vec![0.0; width * height * 3];
        // Interleave XYB channels for batch conversion
        let mut xyb_interleaved = vec![0.0; width * height * 3];
        for i in 0..(width * height) {
            xyb_interleaved[i * 3] = xyb[0][i];     // X
            xyb_interleaved[i * 3 + 1] = xyb[1][i]; // Y
            xyb_interleaved[i * 3 + 2] = xyb[2][i]; // B-Y
        }
        xyb_to_rgb_image_simd(&xyb_interleaved, &mut linear_rgb, width, height);

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

    /// Decode DC coefficients using ANS with differential decoding
    fn decode_dc_coefficients<R: Read>(
        &self,
        reader: &mut BitReader<R>,
    ) -> JxlResult<Vec<i16>> {
        // Read number of DC coefficients
        let num_dc = reader.read_u32(20)? as usize;

        if num_dc == 0 {
            return Ok(Vec::new());
        }

        // Read distribution from bitstream
        let dist = self.read_distribution(reader)?;

        // Read encoded length
        let encoded_len = reader.read_u32(20)? as usize;

        // Read encoded bytes
        let mut encoded = Vec::with_capacity(encoded_len);
        for _ in 0..encoded_len {
            encoded.push(reader.read_bits(8)? as u8);
        }

        // Create ANS decoder
        let mut ans_decoder = RansDecoder::new(encoded)?;

        // Decode differential DC coefficients
        let mut diffs = Vec::with_capacity(num_dc);
        for _ in 0..num_dc {
            let symbol = ans_decoder.decode_symbol(&dist)?;
            let diff = self.map_symbol_to_coeff(symbol, &dist);
            diffs.push(diff);
        }

        // Reconstruct DC coefficients from diffs
        let mut dc_coeffs = Vec::with_capacity(num_dc);
        dc_coeffs.push(diffs[0]);
        for i in 1..num_dc {
            dc_coeffs.push(dc_coeffs[i - 1] + diffs[i]);
        }

        Ok(dc_coeffs)
    }

    /// Decode AC coefficients using ANS with sparse encoding
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

        if non_zero_count == 0 {
            return Ok(ac_coeffs);
        }

        // Read distribution from bitstream
        let dist = self.read_distribution(reader)?;

        // Read positions
        let mut positions = Vec::with_capacity(non_zero_count);
        for _ in 0..non_zero_count {
            positions.push(reader.read_u32(20)? as usize);
        }

        // Read encoded length
        let encoded_len = reader.read_u32(20)? as usize;

        // Read encoded bytes
        let mut encoded = Vec::with_capacity(encoded_len);
        for _ in 0..encoded_len {
            encoded.push(reader.read_bits(8)? as u8);
        }

        // Create ANS decoder
        let mut ans_decoder = RansDecoder::new(encoded)?;

        // Decode values
        let mut values = Vec::with_capacity(non_zero_count);
        for _ in 0..non_zero_count {
            let symbol = ans_decoder.decode_symbol(&dist)?;
            let val = self.map_symbol_to_coeff(symbol, &dist);
            values.push(val);
        }

        // Place values at positions
        for (pos, val) in positions.iter().zip(values.iter()) {
            if *pos < ac_coeffs.len() {
                ac_coeffs[*pos] = *val;
            }
        }

        Ok(ac_coeffs)
    }

    /// Read ANS distribution from bitstream
    ///
    /// Reads alphabet size and min_val to reconstruct a uniform distribution.
    /// In production, this would read the full frequency table.
    fn read_distribution<R: Read>(
        &self,
        reader: &mut BitReader<R>,
    ) -> JxlResult<AnsDistribution> {
        // Read alphabet size
        let alphabet_size = reader.read_u32(12)? as usize;

        // Read min_val (stored as unsigned, shifted by 32768)
        let min_val_unsigned = reader.read_u32(16)?;
        let min_val = (min_val_unsigned as i32 - 32768) as i16;

        // Create distribution with proper min_val
        let dummy_data: Vec<i16> = (0..alphabet_size).map(|i| min_val + i as i16).collect();
        let dist = build_distribution(&dummy_data);

        Ok(dist)
    }

    /// Map ANS symbol index back to i16 coefficient
    fn map_symbol_to_coeff(&self, symbol: usize, dist: &AnsDistribution) -> i16 {
        // Reverse of map_coeff_to_symbol in encoder
        dist.min_val() + symbol as i16
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
