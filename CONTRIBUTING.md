# Contributing to JPEG XL Rust Reference Implementation

Thank you for your interest in contributing to this project!

**Developed by:** Greg Lamberson, Lamco Development (https://www.lamco.ai/)
**Contact:** greg@lamco.io

## About This Project

This is a reference implementation of JPEG XL in Rust, designed to:
- Serve as an educational resource for understanding JPEG XL
- Provide a Rust-based alternative to the C++ reference implementation
- Complement the existing JPEG XL ecosystem

## Upstream Relationship

This implementation is based on the official **libjxl** C++ reference implementation:
- **Upstream Repository**: https://github.com/libjxl/libjxl
- **Specification**: ISO/IEC 18181:2022
- **Official Website**: https://jpeg.org/jpegxl/

### Related Projects

- **[libjxl](https://github.com/libjxl/libjxl)**: Official C++ reference (encoder + decoder)
- **[jxl-oxide](https://github.com/tirr-c/jxl-oxide)**: Production Rust decoder
- **This project**: Educational Rust reference (encoder + decoder)

## How to Contribute

### Areas for Contribution

1. **Code Quality**
   - Improve Rust idioms and patterns
   - Add documentation and examples
   - Enhance type safety

2. **Performance**
   - SIMD optimizations
   - Parallel processing improvements
   - Memory efficiency

3. **Features** (as documented in IMPLEMENTATION.md)
   - Full DC/AC group processing
   - Adaptive quantization
   - Progressive decoding
   - Animation support
   - JPEG reconstruction mode

4. **Testing**
   - Unit tests for individual modules
   - Integration tests
   - Conformance testing against libjxl

### Contribution Process

1. **Fork** this repository
2. **Create** a feature branch (`git checkout -b feature/amazing-feature`)
3. **Commit** your changes with clear messages
4. **Test** your changes (`cargo test --all`)
5. **Push** to your branch (`git push origin feature/amazing-feature`)
6. **Open** a Pull Request

### Code Standards

- Follow Rust 2021 edition conventions
- Use `rustfmt` for formatting
- Use `clippy` for linting
- Add tests for new functionality
- Update documentation as needed

### Commit Messages

Follow conventional commits format:
```
feat: Add support for progressive decoding
fix: Correct ANS entropy decoding edge case
docs: Update encoder usage examples
test: Add unit tests for DCT transform
```

## Questions or Ideas?

- **Issues**: Open an issue for bugs or feature requests
- **Discussions**: For general questions about the implementation
- **Email**: greg@lamco.io for direct contact

## Code of Conduct

- Be respectful and inclusive
- Focus on constructive feedback
- Help make this a welcoming learning resource

## License

By contributing, you agree that your contributions will be licensed under the BSD 3-Clause License, matching the libjxl project.

Copyright (c) 2025 Greg Lamberson, Lamco Development
