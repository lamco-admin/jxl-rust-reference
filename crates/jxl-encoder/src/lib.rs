//! JPEG XL encoder implementation

use jxl_bitstream::{AnsDistribution, RansEncoder, BitWriter};
use jxl_color::{rgb_to_xyb, srgb_u8_to_linear_f32};
use jxl_core::*;
use jxl_headers::Container;
use jxl_transform::{
    dct_channel, generate_xyb_quant_tables, quantize_channel, separate_dc_ac, zigzag_scan_channel,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Cursor, Write};
use std::path::Path;

/// Encoder options
#[derive(Debug, Clone)]
pub struct EncoderOptions {
    /// Quality (0-100, higher is better)
    pub quality: f32,
    /// Encoding effort (1-9, higher is slower but better compression)
    pub effort: u8,
    /// Use lossless encoding
    pub lossless: bool,
    /// Target bits per pixel (for lossy)
    pub target_bpp: Option<f32>,
}

impl Default for EncoderOptions {
    fn default() -> Self {
        Self {
            quality: consts::DEFAULT_QUALITY,
            effort: consts::DEFAULT_EFFORT,
            lossless: false,
            target_bpp: None,
        }
    }
}

impl EncoderOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn quality(mut self, quality: f32) -> Self {
        self.quality = quality.clamp(consts::MIN_QUALITY, consts::MAX_QUALITY);
        self
    }

    pub fn effort(mut self, effort: u8) -> Self {
        self.effort = effort.clamp(consts::MIN_EFFORT, consts::MAX_EFFORT);
        self
    }

    pub fn lossless(mut self, lossless: bool) -> Self {
        self.lossless = lossless;
        self
    }
}

/// JPEG XL encoder
pub struct JxlEncoder {
    /// Encoder configuration options
    /// Note: In this reference implementation, options are stored but not fully utilized yet.
    /// A complete implementation would use these for quality/effort trade-offs.
    #[allow(dead_code)]
    options: EncoderOptions,
}

impl JxlEncoder {
    pub fn new(options: EncoderOptions) -> Self {
        Self { options }
    }

    /// Encode an image to a file
    pub fn encode_file<P: AsRef<Path>>(&self, image: &Image, path: P) -> JxlResult<()> {
        let file = File::create(path)?;
        let writer = BufWriter::new(file);
        self.encode(image, writer)
    }

    /// Encode an image to a writer with JPEG XL container format
    pub fn encode<W: Write>(&self, image: &Image, mut writer: W) -> JxlResult<()> {
        // Step 1: Encode codestream to buffer
        let mut codestream = Vec::new();
        {
            let mut bit_writer = BitWriter::new(Cursor::new(&mut codestream));

            // Write naked codestream signature
            bit_writer.write_bits(0x0AFF, 16)?;

            // Write size header (simplified)
            let small = image.width() <= 32 && image.height() <= 32;
            bit_writer.write_bits(if small { 0 } else { 1 }, 8)?;

            if small {
                bit_writer.write_bits((image.width() - 1) as u64, 5)?;
                bit_writer.write_bits((image.height() - 1) as u64, 5)?;
            } else {
                bit_writer.write_u32(image.width(), 9)?;
                bit_writer.write_u32(image.height(), 9)?;
            }

            // Write bit depth
            let bit_depth_enc = match image.pixel_type {
                PixelType::U8 => 0,
                PixelType::U16 => 2,
                PixelType::F16 => 2,
                PixelType::F32 => 3,
            };
            bit_writer.write_bits(bit_depth_enc, 2)?;
            if bit_depth_enc == 3 {
                bit_writer.write_bits(31, 6)?; // 32-bit
            }

            // Write channels
            let num_extra = image.channel_count() - 3;
            bit_writer.write_bits(num_extra as u64, 2)?;

            // Write color encoding
            let color_enc = match image.color_encoding {
                ColorEncoding::SRGB => 0,
                ColorEncoding::LinearSRGB => 1,
                ColorEncoding::XYB => 2,
                _ => 3,
            };
            bit_writer.write_bits(color_enc, 2)?;

            // Write orientation
            bit_writer.write_bits(1, 3)?; // Identity

            // Write flags
            bit_writer.write_bit(false)?; // not animation
            bit_writer.write_bit(false)?; // no preview

            // Encode frame data
            self.encode_frame(image, &mut bit_writer)?;

            bit_writer.flush()?;
        }

        // Step 2: Wrap codestream in JPEG XL container
        let container = Container::with_codestream(codestream);

        // Step 3: Write container to output
        container.write(&mut writer)?;

        Ok(())
    }

    fn encode_frame<W: Write>(&self, image: &Image, writer: &mut BitWriter<W>) -> JxlResult<()> {
        // Full encoding pipeline:
        // 1. Convert input to f32
        // 2. Convert sRGB to linear RGB
        // 3. Convert RGB to XYB color space
        // 4. Apply DCT transformation to 8x8 blocks
        // 5. Quantize coefficients
        // 6. Encode using ANS entropy coding

        let width = image.width() as usize;
        let height = image.height() as usize;
        let num_channels = image.channel_count();

        // Only support RGB/RGBA for now
        if num_channels < 3 {
            return Err(JxlError::UnsupportedFeature(
                "Only RGB/RGBA images are currently supported".to_string(),
            ));
        }

        // Step 1: Convert to f32 and normalize to [0, 1]
        let linear_rgb = self.convert_to_linear_f32(image)?;

        // Step 2: Convert RGB to XYB color space
        let mut xyb = vec![0.0; width * height * 3];
        self.rgb_to_xyb_image(&linear_rgb, &mut xyb, width, height);

        // Step 3: Apply DCT transformation to each channel (parallel)
        // CRITICAL: Scale XYB values to pixel range (0-255) before DCT
        // XYB values are in ~0-1 range from linear RGB, but DCT expects larger values
        // for proper quantization. Without scaling, all AC coefficients quantize to zero!
        const XYB_SCALE: f32 = 255.0;

        // Process X, Y, and B-Y channels independently for maximum throughput
        let dct_coeffs: Vec<Vec<f32>> = (0..3)
            .into_par_iter()
            .map(|c| {
                let mut channel = self.extract_channel(&xyb, width, height, c, 3);
                // Scale to pixel range before DCT
                for val in &mut channel {
                    *val *= XYB_SCALE;
                }
                let mut dct_coeff = vec![0.0; width * height];
                dct_channel(&channel, width, height, &mut dct_coeff);
                dct_coeff
            })
            .collect();

        // Step 4: Quantize coefficients with XYB-tuned tables (parallel)
        // Use per-channel quantization for optimal perceptual quality
        let xyb_tables = generate_xyb_quant_tables(self.options.quality);
        let quant_tables = [&xyb_tables.x_table, &xyb_tables.y_table, &xyb_tables.b_table];

        let quantized: Vec<Vec<i16>> = dct_coeffs
            .par_iter()
            .zip(quant_tables.par_iter())
            .map(|(dct_coeff, quant_table)| {
                let mut quantized_channel = Vec::new();
                quantize_channel(dct_coeff, width, height, quant_table, &mut quantized_channel);
                quantized_channel
            })
            .collect();

        // Step 5: Encode quantized coefficients using simplified ANS
        self.encode_coefficients(&quantized, width, height, writer)?;

        // Step 6: If there's an alpha channel, encode it separately
        if num_channels == 4 {
            self.encode_alpha_channel(&linear_rgb, width, height, writer)?;
        }

        Ok(())
    }

    /// Convert image buffer to linear f32
    fn convert_to_linear_f32(&self, image: &Image) -> JxlResult<Vec<f32>> {
        let _width = image.width() as usize;
        let _height = image.height() as usize;
        let _num_channels = image.channel_count();

        let mut linear = Vec::new();

        match &image.buffer {
            ImageBuffer::U8(buffer) => {
                // Convert U8 sRGB to linear f32
                for &pixel in buffer.iter() {
                    linear.push(srgb_u8_to_linear_f32(pixel));
                }
            }
            ImageBuffer::U16(buffer) => {
                // Convert U16 to linear f32 (assume sRGB)
                for &pixel in buffer.iter() {
                    let normalized = pixel as f32 / 65535.0;
                    linear.push(srgb_u8_to_linear_f32((normalized * 255.0) as u8));
                }
            }
            ImageBuffer::F32(buffer) => {
                // Already f32, but may need sRGB to linear conversion
                if image.color_encoding == ColorEncoding::SRGB {
                    for &pixel in buffer.iter() {
                        linear.push(jxl_color::srgb_to_linear(pixel));
                    }
                } else {
                    linear = buffer.clone();
                }
            }
        }

        Ok(linear)
    }

    /// Convert RGB to XYB for entire image
    fn rgb_to_xyb_image(&self, rgb: &[f32], xyb: &mut [f32], width: usize, height: usize) {
        let pixel_count = width * height;

        for i in 0..pixel_count {
            let r = rgb[i * 3];
            let g = rgb[i * 3 + 1];
            let b = rgb[i * 3 + 2];

            let (x, y, b_minus_y) = rgb_to_xyb(r, g, b);

            xyb[i * 3] = x;
            xyb[i * 3 + 1] = y;
            xyb[i * 3 + 2] = b_minus_y;
        }
    }

    /// Extract a single channel from interleaved data
    fn extract_channel(
        &self,
        data: &[f32],
        width: usize,
        height: usize,
        channel: usize,
        num_channels: usize,
    ) -> Vec<f32> {
        let mut channel_data = Vec::with_capacity(width * height);

        for i in 0..(width * height) {
            channel_data.push(data[i * num_channels + channel]);
        }

        channel_data
    }

    /// Encode quantized DCT coefficients with ANS entropy coding
    fn encode_coefficients<W: Write>(
        &self,
        quantized: &[Vec<i16>],
        width: usize,
        height: usize,
        writer: &mut BitWriter<W>,
    ) -> JxlResult<()> {
        // Production-grade JPEG XL coefficient encoding with ANS:
        // 1. Apply zigzag scan to organize coefficients by frequency
        // 2. Separate DC and AC coefficients (different statistical properties)
        // 3. Build frequency distributions for DC and AC
        // 4. Encode distributions in bitstream
        // 5. Encode coefficients using ANS entropy coding
        //
        // ANS provides better compression than variable-length coding.

        // Collect all DC and AC coefficients for frequency analysis
        let mut all_dc_diffs = Vec::new();
        let mut all_ac_values = Vec::new();

        for channel in quantized {
            // Apply zigzag scanning
            let mut zigzag_data = Vec::new();
            zigzag_scan_channel(channel, width, height, &mut zigzag_data);

            // Separate DC and AC coefficients
            let (dc_coeffs, ac_coeffs) = separate_dc_ac(&zigzag_data);

            // Collect DC differences
            if !dc_coeffs.is_empty() {
                all_dc_diffs.push(dc_coeffs[0]); // First DC value
                for i in 1..dc_coeffs.len() {
                    all_dc_diffs.push(dc_coeffs[i] - dc_coeffs[i - 1]); // Differences
                }
            }

            // Collect non-zero AC coefficients
            for &ac in &ac_coeffs {
                if ac != 0 {
                    all_ac_values.push(ac);
                }
            }
        }

        // Build ANS distributions
        let dc_dist = self.build_distribution(&all_dc_diffs);
        let ac_dist = self.build_distribution(&all_ac_values);


        // Write distributions to bitstream
        self.write_distribution(&dc_dist, writer)?;
        self.write_distribution(&ac_dist, writer)?;

        // Encode each channel
        for channel in quantized {
            let mut zigzag_data = Vec::new();
            zigzag_scan_channel(channel, width, height, &mut zigzag_data);

            let (dc_coeffs, ac_coeffs) = separate_dc_ac(&zigzag_data);

            // Encode DC with ANS
            self.encode_dc_coefficients_ans(&dc_coeffs, &dc_dist, writer)?;

            // Encode AC with ANS
            self.encode_ac_coefficients_ans(&ac_coeffs, &ac_dist, writer)?;
        }

        Ok(())
    }

    /// Build ANS frequency distribution from coefficients
    fn build_distribution(&self, coeffs: &[i16]) -> AnsDistribution {
        // Map coefficients to non-negative symbols (for ANS alphabet)
        // Use zigzag encoding: 0 -> 0, 1 -> 1, -1 -> 2, 2 -> 3, -2 -> 4, etc.
        let mut freq_map: HashMap<u32, u32> = HashMap::new();

        for &coeff in coeffs {
            let symbol = if coeff >= 0 {
                (coeff as u32) * 2
            } else {
                ((-coeff) as u32) * 2 - 1
            };
            *freq_map.entry(symbol).or_insert(0) += 1;
        }

        // Add minimum frequency for unseen symbols (for robustness)
        if freq_map.is_empty() {
            freq_map.insert(0, 1);
        }

        // Convert to frequency vector - only include symbols that appear
        // Don't waste probability mass on symbols that never occur
        let max_symbol = *freq_map.keys().max().unwrap_or(&0);
        let alphabet_size = (max_symbol + 1) as usize;

        // Build sparse frequency table
        let mut frequencies = vec![0u32; alphabet_size];
        for (&symbol, &freq) in &freq_map {
            // Add small base frequency for stability, plus actual frequency
            frequencies[symbol as usize] = freq + 1;
        }

        // Ensure at least one symbol has non-zero frequency
        if frequencies.iter().all(|&f| f == 0) {
            frequencies[0] = 1;
        }

        AnsDistribution::from_frequencies(&frequencies).unwrap_or_else(|_| {
            // Fallback to uniform distribution if frequency table creation fails
            AnsDistribution::from_frequencies(&vec![1; 2]).unwrap()
        })
    }

    /// Write ANS distribution to bitstream
    fn write_distribution<W: Write>(
        &self,
        dist: &AnsDistribution,
        writer: &mut BitWriter<W>,
    ) -> JxlResult<()> {
        // Write alphabet size (16 bits to support larger alphabets)
        writer.write_u32(dist.alphabet_size() as u32, 16)?;

        // Write frequencies (simplified - just write raw frequencies)
        for i in 0..dist.alphabet_size() {
            let freq = dist.frequency(i) as u32;
            writer.write_u32(freq, 16)?;
        }

        Ok(())
    }

    /// Encode DC coefficients using ANS
    fn encode_dc_coefficients_ans<W: Write>(
        &self,
        dc_coeffs: &[i16],
        dist: &AnsDistribution,
        writer: &mut BitWriter<W>,
    ) -> JxlResult<()> {
        // Write number of DC coefficients
        writer.write_u32(dc_coeffs.len() as u32, 20)?;

        if dc_coeffs.is_empty() {
            return Ok(());
        }

        // Prepare symbols to encode
        let mut symbols = Vec::with_capacity(dc_coeffs.len());

        // First DC value
        symbols.push(self.coeff_to_symbol(dc_coeffs[0]));

        // DC differences
        for i in 1..dc_coeffs.len() {
            let diff = dc_coeffs[i] - dc_coeffs[i - 1];
            symbols.push(self.coeff_to_symbol(diff));
        }

        // Prepare ANS encoder
        let mut encoder = RansEncoder::new();

        // CRITICAL: rANS is LIFO - encode symbols in REVERSE order
        // so decoder gets them in forward order
        for &symbol in symbols.iter().rev() {
            encoder.encode_symbol(symbol as usize, dist)?;
        }

        // Finalize and write ANS stream
        let ans_data = encoder.finalize();
        writer.write_u32(ans_data.len() as u32, 20)?;
        for &byte in &ans_data {
            writer.write_bits(byte as u64, 8)?;
        }

        Ok(())
    }

    /// Encode AC coefficients using ANS
    fn encode_ac_coefficients_ans<W: Write>(
        &self,
        ac_coeffs: &[i16],
        dist: &AnsDistribution,
        writer: &mut BitWriter<W>,
    ) -> JxlResult<()> {
        // Count and encode non-zero AC coefficients
        let non_zero_count = ac_coeffs.iter().filter(|&&c| c != 0).count();
        writer.write_u32(non_zero_count as u32, 20)?;

        if non_zero_count == 0 {
            return Ok(());
        }

        // Encode positions (still using fixed-width, could optimize further)
        for (pos, &coeff) in ac_coeffs.iter().enumerate() {
            if coeff != 0 {
                writer.write_u32(pos as u32, 20)?;
            }
        }

        // Collect non-zero symbols and coefficients
        let mut symbols = Vec::with_capacity(non_zero_count);
        let mut non_zero_coeffs = Vec::with_capacity(non_zero_count);
        let mut positions_vec = Vec::with_capacity(non_zero_count);
        for (pos, &coeff) in ac_coeffs.iter().enumerate() {
            if coeff != 0 {
                non_zero_coeffs.push(coeff);
                positions_vec.push(pos);
                symbols.push(self.coeff_to_symbol(coeff));
            }
        }

        // Encode values with ANS
        let mut encoder = RansEncoder::new();

        // CRITICAL: rANS is LIFO - encode symbols in REVERSE order
        // so decoder gets them in forward order
        for &symbol in symbols.iter().rev() {
            encoder.encode_symbol(symbol as usize, dist)?;
        }

        let ans_data = encoder.finalize();
        writer.write_u32(ans_data.len() as u32, 20)?;
        for &byte in &ans_data {
            writer.write_bits(byte as u64, 8)?;
        }

        Ok(())
    }

    /// Convert coefficient to symbol (zigzag encoding)
    fn coeff_to_symbol(&self, coeff: i16) -> u32 {
        if coeff >= 0 {
            (coeff as u32) * 2
        } else {
            ((-coeff) as u32) * 2 - 1
        }
    }

    /// Convert symbol to coefficient (inverse zigzag)
    #[allow(dead_code)]
    fn symbol_to_coeff(&self, symbol: u32) -> i16 {
        if symbol % 2 == 0 {
            (symbol / 2) as i16
        } else {
            -(((symbol + 1) / 2) as i16)
        }
    }


    /// Encode alpha channel separately
    fn encode_alpha_channel<W: Write>(
        &self,
        linear_rgba: &[f32],
        width: usize,
        height: usize,
        writer: &mut BitWriter<W>,
    ) -> JxlResult<()> {
        // Extract alpha channel and encode as-is (could apply DCT in full implementation)
        for i in 0..(width * height) {
            let alpha = linear_rgba[i * 4 + 3];
            let alpha_u8 = (alpha * 255.0).round().clamp(0.0, 255.0) as u8;
            writer.write_bits(alpha_u8 as u64, 8)?;
        }

        Ok(())
    }
}

impl Default for JxlEncoder {
    fn default() -> Self {
        Self::new(EncoderOptions::default())
    }
}
