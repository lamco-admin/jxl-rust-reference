# JPEG XL Rust Implementation Details

**Developed by:** Greg Lamberson, Lamco Development (https://www.lamco.ai/)
**Contact:** greg@lamco.io

## Overview

This is a reference implementation of JPEG XL (ISO/IEC 18181) in Rust, based on the official libjxl C++ implementation (v0.11.1).

**Reference Repository**: https://github.com/libjxl/libjxl
**This Repository**: https://github.com/lamco-admin/jxl-rust-reference

## Architecture

The implementation is structured as a Cargo workspace with the following crates:

### Core Crates

#### jxl-core
- **Purpose**: Fundamental types and error handling
- **Key Components**:
  - `JxlError` and `JxlResult` types
  - `Image` and `ImageBuffer` structures
  - Color encoding types (`ColorEncoding`, `ColorChannels`)
  - Pixel types (`PixelType`, `Sample` trait)
  - Image metadata structures
  - Constants and configuration

#### jxl-bitstream
- **Purpose**: Bitstream reading/writing and entropy coding
- **Key Components**:
  - `BitReader` and `BitWriter` for bit-level I/O
  - ANS (Asymmetric Numeral Systems) encoder/decoder
  - Huffman coding support
- **Algorithms**: Implements the ANS entropy coding used in JPEG XL for efficient compression

#### jxl-color
- **Purpose**: Color space transformations
- **Key Components**:
  - XYB color space (JPEG XL's perceptual color space)
  - sRGB ↔ Linear RGB conversions
  - Color correlation transforms (YCoCg)
- **Details**: XYB is a perceptual color space designed to be more uniform than RGB

#### jxl-transform
- **Purpose**: Image transformations
- **Key Components**:
  - DCT (Discrete Cosine Transform) - 8x8 blocks
  - Prediction modes (Left, Top, Average, Paeth, Gradient)
  - Quantization for lossy compression
- **Algorithms**: Implements DCT-II (forward) and DCT-III (inverse)

#### jxl-headers
- **Purpose**: Header parsing and generation
- **Key Components**:
  - `JxlHeader` structure
  - Bitstream signature validation
  - Metadata parsing

#### jxl-decoder
- **Purpose**: JPEG XL decoding pipeline
- **Key Components**:
  - `JxlDecoder` main API
  - Frame decoding
  - Integration with all transform and color components
- **Process**:
  1. Parse header
  2. Decode entropy-coded bitstream (ANS)
  3. Dequantize coefficients
  4. Apply inverse DCT
  5. Convert from XYB to RGB color space

#### jxl-encoder
- **Purpose**: JPEG XL encoding pipeline
- **Key Components**:
  - `JxlEncoder` main API
  - `EncoderOptions` for quality/effort control
  - Frame encoding
- **Process**:
  1. Convert RGB to XYB color space
  2. Apply DCT transformation
  3. Quantize coefficients (lossy mode)
  4. Entropy encode using ANS
  5. Write header and bitstream

#### jxl (main crate)
- **Purpose**: High-level API
- **Exports**: All public types and functions from sub-crates
- **Documentation**: Main entry point with examples

## JPEG XL Format Details

### File Structure
```
┌─────────────────────┐
│   Signature (0x0AFF)│
├─────────────────────┤
│   Size Header       │
├─────────────────────┤
│   Image Header      │
├─────────────────────┤
│   Frame Data        │
│   - DC Groups       │
│   - AC Groups       │
└─────────────────────┘
```

### Key Features Implemented

1. **Bitstream Format**:
   - Variable-length encoding for dimensions
   - Bit depth support (8, 10, 12, 16, 32-bit)
   - Multiple channel support (1-4 channels)

2. **Color Spaces**:
   - sRGB (with gamma correction)
   - Linear sRGB
   - XYB (perceptual color space)

3. **Compression**:
   - ANS entropy coding
   - DCT transformation (8x8 blocks)
   - Quantization with quality parameter
   - Prediction modes

4. **Image Features**:
   - Multiple pixel types (u8, u16, f32)
   - Orientation metadata
   - Animation support (structure in place)

## Implementation Status

### Completed
- ✅ Core types and error handling
- ✅ Bitstream reader/writer
- ✅ ANS entropy coding
- ✅ Huffman coding
- ✅ XYB color space transforms
- ✅ sRGB transforms
- ✅ Color correlation
- ✅ DCT (8x8)
- ✅ Prediction modes
- ✅ Quantization
- ✅ Header parsing/writing
- ✅ Basic encoder/decoder framework

### Simplified/Partial
- ⚠️ Frame decoding (simplified for reference)
- ⚠️ DC/AC group processing (basic implementation)
- ⚠️ Parallel processing hooks (rayon dependency in place)

### To Be Completed (for full conformance)
- ❌ Full DC group processing (2048x2048 regions)
- ❌ Full AC group processing (256x256 regions)
- ❌ Adaptive quantization
- ❌ Noise synthesis
- ❌ Patches
- ❌ Splines
- ❌ Progressive decoding
- ❌ Animation playback
- ❌ JPEG reconstruction mode
- ❌ Full ICC profile support
- ❌ EXIF/XMP metadata handling

## Performance Considerations

1. **Parallelism**: Uses Rayon for potential multi-threading
2. **Memory**: Zero-copy where possible
3. **SIMD**: Could be added for DCT and color transforms

## Testing

Run tests with:
```bash
cargo test --all
```

Build examples:
```bash
cargo build --examples
```

Run example:
```bash
cargo run --example encode_decode
```

## References

- [JPEG XL Official Site](https://jpeg.org/jpegxl/)
- [libjxl Reference Implementation](https://github.com/libjxl/libjxl)
- [ISO/IEC 18181:2022 Standard](https://www.iso.org/standard/77977.html)
- [JPEG XL Whitepaper](https://ds.jpeg.org/whitepapers/jpeg-xl-whitepaper.pdf)

## License

BSD 3-Clause License (matching libjxl)
