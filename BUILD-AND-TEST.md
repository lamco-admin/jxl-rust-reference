# Building and Testing - JPEG XL Rust Reference

**Developer:** Greg Lamberson, Lamco Development (https://www.lamco.ai/)

## Prerequisites

### Required

- **Rust**: 1.85.0 or newer (2024 edition)
- **Cargo**: Comes with Rust
- **Git**: For cloning the repository

### Installation

#### Install Rust (if not already installed)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

#### Verify Installation

```bash
rustc --version  # Should show 1.85.0 or newer
cargo --version
```

#### Update Rust (if needed)

```bash
rustup update
```

## Getting the Code

### Clone the Repository

```bash
git clone https://github.com/lamco-admin/jxl-rust-reference.git
cd jxl-rust-reference
```

### Repository Structure

```
jxl-rust-reference/
├── Cargo.toml                 # Workspace configuration
├── Cargo.lock                 # Dependency lock file
├── README.md                  # Project overview
├── IMPLEMENTATION.md          # Technical details
├── LIMITATIONS.md             # ⚠️ Important scope documentation
├── CONTRIBUTING.md            # Contribution guidelines
├── BUILD-AND-TEST.md          # This file
├── EVALUATION.md              # Implementation evaluation
├── crates/                    # Workspace crates
│   ├── jxl-core/             # Core types and errors
│   ├── jxl-bitstream/        # Bitstream I/O and ANS
│   ├── jxl-color/            # Color transformations
│   ├── jxl-transform/        # DCT and prediction
│   ├── jxl-headers/          # Header parsing
│   ├── jxl-decoder/          # Decoder implementation
│   ├── jxl-encoder/          # Encoder implementation
│   └── jxl/                  # High-level API
└── examples/                  # Example programs
    └── encode_decode.rs
```

## Building

### Quick Build (Debug)

```bash
cargo build
```

- Compiles in debug mode (unoptimized)
- Fast compilation
- Larger binaries with debug symbols
- Suitable for development

### Release Build (Optimized)

```bash
cargo build --release
```

- Compiles with optimizations
- Slower compilation
- Smaller, faster binaries
- Use for performance testing

### Build Specific Crate

```bash
# Build just the core crate
cargo build -p jxl-core

# Build just the encoder
cargo build -p jxl-encoder

# Build with verbose output
cargo build --verbose
```

### Check Without Building

```bash
# Fast compilation check (no code generation)
cargo check

# Check all workspace members
cargo check --workspace

# Check all targets (lib, examples, tests)
cargo check --all-targets
```

### Clean Build Artifacts

```bash
# Remove target directory
cargo clean

# Check disk space used
du -sh target/
```

## Testing

### Run All Tests

```bash
cargo test
```

### Test Specific Crate

```bash
# Test core crate only
cargo test -p jxl-core

# Test bitstream crate
cargo test -p jxl-bitstream

# Test with output
cargo test -- --nocapture
```

### Test Specific Function

```bash
# Test a specific test function
cargo test test_image_creation

# Test with pattern matching
cargo test color

# Show test names without running
cargo test -- --list
```

### Run Tests with Details

```bash
# Show test output even for passing tests
cargo test -- --nocapture --test-threads=1

# Show all test results
cargo test --verbose
```

### Test Coverage (requires tarpaulin)

```bash
# Install tarpaulin
cargo install cargo-tarpaulin

# Run coverage
cargo tarpaulin --out Html --output-dir coverage/
```

## Code Quality

### Format Code (rustfmt)

```bash
# Format all code
cargo fmt

# Check if formatting is needed (CI mode)
cargo fmt -- --check

# Format specific file
rustfmt crates/jxl-core/src/types.rs
```

### Lint Code (clippy)

```bash
# Run clippy (linter)
cargo clippy

# Run clippy for all targets
cargo clippy --all-targets

# Treat warnings as errors (strict mode)
cargo clippy -- -D warnings

# Auto-fix issues where possible
cargo clippy --fix
```

### Check Everything

```bash
# Format, lint, and test in one go
cargo fmt && cargo clippy --all-targets -- -D warnings && cargo test
```

## Running Examples

### Basic Example

```bash
# Run the encode_decode example
cargo run --example encode_decode

# Run with release optimizations
cargo run --release --example encode_decode
```

### List Available Examples

```bash
ls examples/
cargo build --examples
```

### Build Example Binary

```bash
# Build example (creates target/debug/examples/encode_decode)
cargo build --example encode_decode

# Run built binary directly
./target/debug/examples/encode_decode
```

## Development Workflow

### Recommended Workflow

```bash
# 1. Make changes to code
$EDITOR crates/jxl-core/src/types.rs

# 2. Quick check (fast)
cargo check

# 3. Run tests for affected crate
cargo test -p jxl-core

# 4. Format code
cargo fmt

# 5. Run linter
cargo clippy

# 6. Run all tests
cargo test

# 7. Build release if needed
cargo build --release
```

### Watch Mode (requires cargo-watch)

```bash
# Install cargo-watch
cargo install cargo-watch

# Auto-run checks on file changes
cargo watch -x check

# Auto-run tests on changes
cargo watch -x test

# Auto-run multiple commands
cargo watch -x check -x test -x clippy
```

## Benchmarking

### Basic Benchmarking

```bash
# Build in release mode first
cargo build --release

# Time example execution
time ./target/release/examples/encode_decode
```

### Criterion Benchmarks (if implemented)

```bash
# Install criterion support
# Add criterion to Cargo.toml

# Run benchmarks
cargo bench
```

## Documentation

### Generate Documentation

```bash
# Build documentation
cargo doc

# Build and open in browser
cargo doc --open

# Include private items
cargo doc --document-private-items

# Build for all workspace members
cargo doc --workspace
```

### Documentation Location

Generated docs: `target/doc/jxl/index.html`

## Dependency Management

### Update Dependencies

```bash
# Check for outdated dependencies
cargo outdated  # requires cargo-outdated

# Update dependencies
cargo update

# Update specific dependency
cargo update -p thiserror
```

### Audit Dependencies

```bash
# Install cargo-audit
cargo install cargo-audit

# Check for security vulnerabilities
cargo audit
```

### Show Dependency Tree

```bash
# Show dependency tree
cargo tree

# Show dependencies for specific crate
cargo tree -p jxl-core

# Show reverse dependencies
cargo tree -i jxl-core
```

## Troubleshooting

### Common Issues

#### Issue: "Rust version too old"

```bash
# Update Rust
rustup update stable
rustc --version  # Verify >= 1.85.0
```

#### Issue: "Dependency resolution failed"

```bash
# Update Cargo.lock
cargo update

# Clean and rebuild
cargo clean
cargo build
```

#### Issue: "Tests failing"

```bash
# Run with verbose output
cargo test -- --nocapture

# Run specific failing test
cargo test test_name -- --nocapture

# Check if clippy has suggestions
cargo clippy
```

#### Issue: "Clippy warnings"

```bash
# See all warnings
cargo clippy --all-targets

# Auto-fix where possible
cargo clippy --fix --allow-dirty --allow-staged
```

### Getting Help

```bash
# Cargo help
cargo --help
cargo build --help
cargo test --help

# Rust documentation
rustup doc
rustup doc --book
```

## CI/CD Integration

### GitHub Actions Example

See `.github/workflows/rust.yml` for the full CI configuration.

### Manual CI Simulation

```bash
# Run the same checks as CI
./scripts/ci-check.sh  # if provided

# Or manually:
cargo fmt -- --check && \
cargo clippy --all-targets -- -D warnings && \
cargo test --all && \
cargo build --release
```

## Performance Profiling

### Basic Profiling (Linux)

```bash
# Install perf tools
sudo apt-get install linux-tools-generic

# Build with debug symbols
cargo build --release

# Profile example
perf record ./target/release/examples/encode_decode
perf report
```

### Flamegraph (requires cargo-flamegraph)

```bash
# Install flamegraph
cargo install flamegraph

# Generate flamegraph
cargo flamegraph --example encode_decode
```

## Cross-Compilation

### Target Other Platforms

```bash
# List installed targets
rustup target list --installed

# Add target
rustup target add x86_64-pc-windows-gnu

# Build for target
cargo build --target x86_64-pc-windows-gnu
```

## Advanced Cargo Commands

### Useful Commands

```bash
# Show build dependencies
cargo build --build-plan

# Explain a compilation error in detail
cargo explain E0502

# Expand macros
cargo expand  # requires cargo-expand

# Show generated assembly
cargo asm  # requires cargo-asm

# Check binary size
cargo bloat --release  # requires cargo-bloat
```

## Workspace Operations

### Workspace Commands

```bash
# Build all workspace members
cargo build --workspace

# Test all workspace members
cargo test --workspace

# Clean all workspace members
cargo clean --workspace

# Update all workspace members
cargo update --workspace
```

## Quick Reference

### Most Common Commands

```bash
# Development
cargo check              # Fast compile check
cargo build              # Build debug
cargo test               # Run tests
cargo run --example X    # Run example

# Quality
cargo fmt                # Format code
cargo clippy             # Lint code

# Release
cargo build --release    # Optimized build
cargo test --release     # Test with optimizations

# Documentation
cargo doc --open         # Generate and view docs

# Maintenance
cargo clean              # Remove build artifacts
cargo update             # Update dependencies
```

### Environment Variables

```bash
# Increase compiler verbosity
RUSTFLAGS="-V" cargo build

# Use more parallel jobs
CARGO_BUILD_JOBS=8 cargo build

# Show build timings
cargo build --timings
```

## Best Practices

### Before Committing

```bash
# Always run before git commit:
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

### Code Review Checklist

- [ ] Code is formatted (`cargo fmt`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] All tests pass (`cargo test`)
- [ ] Documentation is updated
- [ ] CHANGELOG is updated (if applicable)
- [ ] Examples work (`cargo run --example ...`)

## Resources

### Official Documentation
- [The Rust Book](https://doc.rust-lang.org/book/)
- [Cargo Book](https://doc.rust-lang.org/cargo/)
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/)

### This Project
- [README.md](README.md) - Project overview
- [IMPLEMENTATION.md](IMPLEMENTATION.md) - Technical details
- [LIMITATIONS.md](LIMITATIONS.md) - **Important scope information**
- [CONTRIBUTING.md](CONTRIBUTING.md) - How to contribute

### External Resources
- [libjxl](https://github.com/libjxl/libjxl) - Official C++ reference
- [jxl-oxide](https://github.com/tirr-c/jxl-oxide) - Production Rust decoder
- [JPEG XL Specification](https://jpeg.org/jpegxl/documentation.html)

## Contact

- **Developer:** Greg Lamberson
- **Email:** greg@lamco.io
- **Organization:** Lamco Development (https://www.lamco.ai/)
- **Repository:** https://github.com/lamco-admin/jxl-rust-reference

---

**Note:** Remember to read [LIMITATIONS.md](LIMITATIONS.md) to understand the scope of this reference implementation.
