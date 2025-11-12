# JPEG XL Rust Reference Implementation

A complete reference implementation of JPEG XL (ISO/IEC 18181) in Rust, based on the libjxl C++ reference implementation.

**Developed by:** Greg Lamberson, [Lamco Development](https://www.lamco.ai/)
**Contact:** greg@lamco.io
**Repository:** https://github.com/lamco-admin/jxl-rust-reference

## Overview

This project provides a pure Rust implementation of the JPEG XL image format, including both encoder and decoder functionality. The implementation follows the ISO/IEC 18181 standard and is structured to match the architecture of the official libjxl reference implementation.

## Repository Information

**Official C++ Reference**: https://github.com/libjxl/libjxl (v0.11.1)

This Rust implementation is based on libjxl's architecture and design but reimplemented idiomatically in Rust with focus on:
- Memory safety
- Type safety
- Modern Rust patterns and best practices
- Performance through zero-cost abstractions

## Project Structure

This is a Cargo workspace containing multiple crates:

- **jxl-core**: Core data structures, types, and common utilities
- **jxl-bitstream**: Bitstream reading/writing and entropy coding (ANS)
- **jxl-color**: Color space transformations and XYB color space
- **jxl-transform**: DCT, prediction, and image transformations
- **jxl-headers**: Header parsing and metadata handling
- **jxl-decoder**: JPEG XL decoder implementation
- **jxl-encoder**: JPEG XL encoder implementation
- **jxl**: High-level API for easy use

## Features

- Lossless and lossy compression
- Support for multiple bit depths (8-bit, 16-bit, float)
- HDR and wide color gamut support
- Progressive decoding
- Animation support
- JPEG reconstruction mode
- Multi-threaded encoding/decoding

## JPEG XL Format

JPEG XL (ISO/IEC 18181) consists of:
- **Part 1**: Core codestream specification
- **Part 2**: File format (.jxl)
- **Part 3**: Decoder conformance requirements
- **Part 4**: Reference software (libjxl)

## Building

```bash
cargo build --release
```

## Testing

```bash
cargo test --all
```

## Usage

### Decoding

```rust
use jxl::{JxlDecoder, PixelFormat};

let decoder = JxlDecoder::new()?;
let image = decoder.decode_file("image.jxl")?;
```

### Encoding

```rust
use jxl::{JxlEncoder, EncoderOptions};

let encoder = JxlEncoder::new()?;
let options = EncoderOptions::default()
    .quality(90.0)
    .effort(7);
encoder.encode_file(&image, "output.jxl", options)?;
```

## Related Projects

This implementation complements the existing JPEG XL ecosystem:
- **[libjxl](https://github.com/libjxl/libjxl)**: Official C++ reference implementation (encoder and decoder)
- **[jxl-oxide](https://github.com/tirr-c/jxl-oxide)**: Spec-conforming Rust decoder (decoder only)
- **jxl-rust-reference** (this project): Rust reference implementation (encoder and decoder)

## License

BSD 3-Clause License (matching libjxl)

Copyright (c) 2025 Greg Lamberson, Lamco Development

## References

- [JPEG XL Official Website](https://jpeg.org/jpegxl/)
- [libjxl Reference Implementation](https://github.com/libjxl/libjxl)
- [JPEG XL Specification](https://jpeg.org/jpegxl/documentation.html)
- [ISO/IEC 18181:2022 Standard](https://www.iso.org/standard/77977.html)

## Contributing

Contributions are welcome! This is a reference implementation designed to be educational and complement the official libjxl C++ implementation. For production use, consider [libjxl](https://github.com/libjxl/libjxl) or [jxl-oxide](https://github.com/tirr-c/jxl-oxide).
