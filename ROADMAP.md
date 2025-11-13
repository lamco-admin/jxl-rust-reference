# JPEG XL Rust Implementation - Development Roadmap

**Project:** jxl-rust-reference
**Developer:** Greg Lamberson, Lamco Development
**Current Status:** ~70% spec compliance (production-capable)
**Goal:** 100% spec compliance, production-ready Rust JPEG XL encoder/decoder
**Last Updated:** November 13, 2025

---

## Overview

This roadmap outlines the path from the current state (~70% spec compliance) to a production-ready, fully spec-compliant JPEG XL encoder/decoder. Development is organized into three phases spanning 1-12 months.

### Current State Summary

**✅ Completed (70% of spec):**
- Production XYB color space (libjxl matrices)
- SIMD-optimized DCT/IDCT (AVX2/NEON)
- XYB-tuned adaptive quantization
- ISO/IEC 18181-2 container format
- Production frame headers
- Modular mode (lossless, 7 predictors)
- Progressive decoding framework
- Parallel processing (Rayon, 2.3× speedup)
- 64 tests passing, zero warnings

**⚠️ Partially Complete:**
- ANS entropy coding (simple distributions only)

**❌ Missing (30% of spec):**
- Full ANS with context modeling
- Conformance testing
- Group-level parallelism
- Streaming API
- Animation
- JPEG reconstruction
- HDR support

---

## Phase 1: Core Completion (1-2 Months)

**Goal:** 85% spec compliance, production viability
**Target Release:** v0.5.0
**Timeline:** 140-180 hours (~1-2 months at 20 hours/week)
**Status:** 🟡 Ready to start

### Critical Priority Tasks

#### 1.1 Complete ANS Entropy Coding ⭐⭐⭐⭐⭐

**Effort:** 40-80 hours
**Impact:** 🔴 **CRITICAL** - 2× compression improvement, spec compliance
**Dependencies:** None
**Current Status:** Partial (simple distributions working)

**Objectives:**
1. Fix complex frequency distribution handling
2. Implement context modeling for coefficients
3. Add adaptive distribution updates
4. Integrate into encoder/decoder pipelines

**Tasks:**
- [ ] Debug differential decoding in `RansDecoder` (crates/jxl-bitstream/src/ans.rs)
- [ ] Fix complex distribution test (currently ignored)
- [ ] Implement context modeling:
  - [ ] DC coefficient contexts (3-5 contexts)
  - [ ] AC coefficient contexts (8-12 contexts based on position)
  - [ ] Adaptive context selection
- [ ] Add distribution normalization
- [ ] Implement hybrid rANS/tANS approach:
  - [ ] Use rANS for DC (better compression for biased distributions)
  - [ ] Use tANS for AC (better performance for sparse data)
- [ ] Add comprehensive tests:
  - [ ] Complex distributions
  - [ ] Context modeling
  - [ ] Roundtrip with full ANS
- [ ] Profile and optimize performance
- [ ] Update documentation

**Expected Outcomes:**
- 2× compression improvement (0.36 BPP → 0.18 BPP)
- All ANS tests passing
- Spec-compliant entropy coding
- Foundation for 85%+ compliance

**Validation:**
- [ ] All ANS tests pass (including previously ignored)
- [ ] Compression ratio improves by 50%+
- [ ] Roundtrip tests maintain PSNR

---

#### 1.2 Add Conformance Testing ⭐⭐⭐⭐⭐

**Effort:** 20-40 hours
**Impact:** 🔴 **CRITICAL** - Validates spec compliance
**Dependencies:** ANS completion (partial)
**Current Status:** Not started

**Objectives:**
1. Validate outputs against libjxl
2. Add conformance test suite
3. CI integration for regression prevention

**Tasks:**
- [ ] Set up conformance testing infrastructure:
  - [ ] Download libjxl test suite
  - [ ] Create test harness for cross-validation
  - [ ] Add comparison utilities (PSNR, SSIM, bitwise)
- [ ] Implement conformance tests:
  - [ ] Encode with this implementation, decode with libjxl
  - [ ] Encode with libjxl, decode with this implementation (when possible)
  - [ ] Compare outputs (pixel-by-pixel validation)
- [ ] Test categories:
  - [ ] Solid colors (simple case)
  - [ ] Gradients (smooth transitions)
  - [ ] High-frequency content (complex textures)
  - [ ] Different resolutions (64×64, 256×256, 1024×1024)
  - [ ] Different quality levels (10, 50, 90, 100/lossless)
  - [ ] Different bit depths (8-bit, 16-bit, float)
- [ ] CI integration:
  - [ ] GitHub Actions workflow
  - [ ] Automated conformance testing on PR
  - [ ] Regression detection
- [ ] Document conformance status:
  - [ ] Create CONFORMANCE.md
  - [ ] List passing/failing test cases
  - [ ] Track progress toward 100% conformance

**Expected Outcomes:**
- Validated spec compliance
- Confidence in interoperability
- Automated regression prevention
- Clear gap identification

**Validation:**
- [ ] 80%+ of conformance tests pass
- [ ] CI pipeline green for conformance
- [ ] Documentation updated with conformance status

---

#### 1.3 Implement Group-Level Parallelism ⭐⭐⭐⭐

**Effort:** 40-60 hours
**Impact:** 🟠 **HIGH** - 5-10× performance improvement
**Dependencies:** None
**Current Status:** Not started (channel-level parallelism only)

**Objectives:**
1. Implement tile/group-based processing
2. Utilize 16+ CPU cores effectively
3. Match libjxl parallelism architecture

**Tasks:**
- [ ] Design group processing architecture:
  - [ ] Define group sizes (matching JPEG XL spec)
  - [ ] DC groups: 2048×2048 regions
  - [ ] AC groups: 256×256 regions
  - [ ] Group dependency analysis
- [ ] Implement group splitting:
  - [ ] Image → DC groups
  - [ ] Image → AC groups
  - [ ] Handle edge cases (partial groups)
- [ ] Parallel group encoding:
  - [ ] Parallel DC group processing
  - [ ] Parallel AC group processing within DC groups
  - [ ] Rayon thread pool configuration
  - [ ] Work stealing for load balancing
- [ ] Parallel group decoding:
  - [ ] Parallel DC group decode
  - [ ] Parallel AC group decode
  - [ ] Result merging
- [ ] Optimize memory usage:
  - [ ] Shared buffer pools
  - [ ] Reduce allocations
  - [ ] Cache-friendly access patterns
- [ ] Benchmarking:
  - [ ] Measure speedup vs. current channel-parallel
  - [ ] Test scalability (1, 2, 4, 8, 16+ cores)
  - [ ] Profile bottlenecks
- [ ] Add group-level tests:
  - [ ] Single group
  - [ ] Multiple groups
  - [ ] Edge cases
  - [ ] Roundtrip validation

**Expected Outcomes:**
- 5-10× performance improvement (0.5 MP/s → 2.5-5 MP/s)
- Efficient multi-core utilization
- Scalable to 16+ cores
- Reduced gap vs. libjxl (40× → 4-8×)

**Validation:**
- [ ] Benchmark shows 5-10× speedup
- [ ] Scales linearly to 8+ cores
- [ ] All roundtrip tests pass
- [ ] Memory usage acceptable

---

### Phase 1 Deliverable: v0.5.0

**Target Metrics:**
- ✅ 85% spec compliance
- ✅ 0.18 BPP compression (2× better than current)
- ✅ 2.5-5 MP/s encoding (10× faster than current)
- ✅ 80%+ conformance tests passing
- ✅ Production-viable for small-medium images

**Release Criteria:**
- [ ] All ANS tests passing
- [ ] Conformance testing infrastructure in place
- [ ] 80%+ conformance tests pass
- [ ] Group-level parallelism implemented
- [ ] Benchmarks show 10× overall speedup
- [ ] Documentation updated (LIMITATIONS.md, README.md, CHANGELOG.md)
- [ ] CI/CD pipeline green

---

## Phase 2: Production Features (3-4 Months)

**Goal:** 95% spec compliance, production-ready
**Target Release:** v0.9.0
**Timeline:** 220-340 hours (~3-4 months at 20 hours/week)
**Status:** 🔴 Not started (depends on Phase 1)

### High-Priority Tasks

#### 2.1 Implement Streaming API ⭐⭐⭐⭐

**Effort:** 80-120 hours
**Impact:** 🟠 **HIGH** - Large image support
**Dependencies:** Group-level parallelism (Phase 1)

**Objectives:**
1. Support large images without full memory load
2. Tile-based processing
3. Memory-efficient encoding/decoding

**Tasks:**
- [ ] Design streaming architecture:
  - [ ] Tile-based processing model
  - [ ] Progressive output writing
  - [ ] Buffered input reading
- [ ] Implement streaming encoder:
  - [ ] `StreamingEncoder` trait/API
  - [ ] Tile iteration and encoding
  - [ ] Output buffer management
  - [ ] Memory limits and backpressure
- [ ] Implement streaming decoder:
  - [ ] `StreamingDecoder` trait/API
  - [ ] Partial bitstream parsing
  - [ ] Tile decoding on demand
  - [ ] Output tile delivery
- [ ] Memory optimization:
  - [ ] Tile buffer pooling
  - [ ] Lazy allocation
  - [ ] Configurable memory limits
- [ ] Add streaming tests:
  - [ ] Large images (4K, 8K)
  - [ ] Memory usage validation
  - [ ] Progressive output verification
- [ ] API examples and documentation

**Expected Outcomes:**
- Support for 4K/8K+ images
- Constant memory usage (independent of image size)
- Production-ready for large image workflows

**Validation:**
- [ ] Successfully encode/decode 8K images
- [ ] Memory usage stays under 500 MB
- [ ] API examples work correctly

---

#### 2.2 Implement Animation Support ⭐⭐⭐

**Effort:** 40-60 hours
**Impact:** 🟡 **MEDIUM** - Major feature for some use cases
**Dependencies:** Streaming API (partial)

**Objectives:**
1. Multi-frame encoding/decoding
2. Frame blending and timing
3. Animation metadata

**Tasks:**
- [ ] Extend frame header handling:
  - [ ] Multi-frame metadata
  - [ ] Frame duration/timing
  - [ ] Blending modes (replace, blend, add)
  - [ ] Frame references
- [ ] Implement multi-frame encoder:
  - [ ] Frame sequence API
  - [ ] Inter-frame optimization
  - [ ] Keyframe selection
- [ ] Implement multi-frame decoder:
  - [ ] Frame sequence decoding
  - [ ] Frame caching
  - [ ] Blending application
  - [ ] Timing control
- [ ] Add animation tests:
  - [ ] Simple 2-frame animation
  - [ ] Complex multi-frame (10+ frames)
  - [ ] Different blending modes
  - [ ] Timing accuracy
- [ ] Examples and documentation

**Expected Outcomes:**
- Full animation support
- Efficient inter-frame compression
- Production-ready for animation workflows

**Validation:**
- [ ] Encode/decode 10-frame animation
- [ ] Blending modes work correctly
- [ ] Timing matches specification

---

#### 2.3 Advanced Compression Tools ⭐⭐⭐

**Effort:** 60-100 hours
**Impact:** 🟡 **MEDIUM** - Better compression for specific content
**Dependencies:** None

**Objectives:**
1. Implement patches (repeating patterns)
2. Implement splines (smooth gradients)
3. Implement noise synthesis

**Tasks:**
- [ ] **Patches** (20-30 hours):
  - [ ] Pattern detection algorithm
  - [ ] Patch encoding/decoding
  - [ ] Reference patch storage
  - [ ] Position encoding
  - [ ] Tests for repeating content
- [ ] **Splines** (25-40 hours):
  - [ ] Gradient detection
  - [ ] Spline fitting (cubic/bezier)
  - [ ] Spline rendering
  - [ ] Tests for smooth gradients
- [ ] **Noise Synthesis** (15-30 hours):
  - [ ] Noise detection and removal
  - [ ] Noise parameters encoding
  - [ ] Noise synthesis on decode
  - [ ] Tests for noisy images
- [ ] Integration:
  - [ ] Mode selection heuristics
  - [ ] Encoder option flags
  - [ ] Performance benchmarks
- [ ] Documentation

**Expected Outcomes:**
- Better compression for specific content types
- 10-30% improvement on applicable images
- Spec-compliant advanced modes

**Validation:**
- [ ] Patches improve compression on repeating patterns
- [ ] Splines improve compression on gradients
- [ ] Noise synthesis preserves perceptual quality

---

### Phase 2 Deliverable: v0.9.0

**Target Metrics:**
- ✅ 95% spec compliance
- ✅ 0.15-0.18 BPP compression (matches libjxl)
- ✅ 5-10 MP/s encoding (2-4× slower than libjxl)
- ✅ 95%+ conformance tests passing
- ✅ Production-ready for most use cases

**Release Criteria:**
- [ ] Streaming API functional
- [ ] Animation support complete
- [ ] Advanced compression tools implemented
- [ ] 95%+ conformance tests pass
- [ ] Large image support validated
- [ ] Performance within 2-4× of libjxl
- [ ] Documentation complete
- [ ] Production deployment examples

---

## Phase 3: Full Spec Compliance (5-12 Months)

**Goal:** 100% spec compliance, feature parity with libjxl
**Target Release:** v1.0.0
**Timeline:** 220-300 hours (~5-12 months at 20 hours/week, lower priority items)
**Status:** 🔴 Not started (depends on Phase 2)

### Remaining Features

#### 3.1 JPEG Reconstruction Mode ⭐⭐⭐

**Effort:** 80-120 hours
**Impact:** 🟡 **MEDIUM** - Major feature for JPEG workflows
**Dependencies:** None

**Objectives:**
1. Lossless JPEG recompression
2. 30-50% JPEG size savings
3. Perfect reconstruction

**Tasks:**
- [ ] JPEG parsing:
  - [ ] JPEG bitstream parser
  - [ ] Extract quantization tables
  - [ ] Extract DCT coefficients
  - [ ] Extract metadata
- [ ] JPEG XL encoding from JPEG:
  - [ ] Map JPEG DCT to JPEG XL DCT
  - [ ] Store original quantization
  - [ ] Preserve JPEG structure
  - [ ] Add reconstruction metadata
- [ ] JPEG reconstruction on decode:
  - [ ] Reconstruct exact JPEG bitstream
  - [ ] Verify bit-perfect output
- [ ] Tests:
  - [ ] Various JPEG files
  - [ ] Bit-perfect reconstruction validation
  - [ ] Compression ratio measurement
- [ ] Documentation and examples

**Expected Outcomes:**
- Lossless JPEG recompression
- 30-50% JPEG size savings
- Bit-perfect reconstruction

**Validation:**
- [ ] Roundtrip JPEG matches original (bit-perfect)
- [ ] Compression ratio: 30-50% savings
- [ ] Works with various JPEG files

---

#### 3.2 HDR Support ⭐⭐

**Effort:** 60-80 hours
**Impact:** 🟢 **LOW-MEDIUM** - Important for HDR workflows
**Dependencies:** None

**Objectives:**
1. PQ (Perceptual Quantizer) transfer function
2. HLG (Hybrid Log-Gamma) transfer function
3. Wide color gamut (Display P3, Rec. 2020)

**Tasks:**
- [ ] Implement transfer functions:
  - [ ] PQ (SMPTE ST 2084)
  - [ ] HLG (Rec. 2100)
  - [ ] Forward/inverse transforms
- [ ] Wide color gamut support:
  - [ ] Display P3 primaries
  - [ ] Rec. 2020 primaries
  - [ ] Color space conversion
- [ ] HDR metadata:
  - [ ] MaxCLL (Content Light Level)
  - [ ] MaxFALL (Frame-Average Light Level)
  - [ ] Mastering display metadata
- [ ] Tests:
  - [ ] HDR10 content
  - [ ] HLG content
  - [ ] Wide gamut images
- [ ] Documentation

**Expected Outcomes:**
- Full HDR support
- Wide color gamut
- Spec-compliant HDR encoding

**Validation:**
- [ ] HDR10 images encode/decode correctly
- [ ] HLG images encode/decode correctly
- [ ] Wide gamut preserved

---

#### 3.3 Full Metadata Support ⭐⭐

**Effort:** 40-60 hours
**Impact:** 🟢 **LOW** - Convenience feature
**Dependencies:** None

**Objectives:**
1. EXIF/XMP integration
2. ICC profile handling
3. Custom metadata boxes

**Tasks:**
- [ ] EXIF integration:
  - [ ] EXIF parsing
  - [ ] EXIF box encoding
  - [ ] EXIF preservation on roundtrip
- [ ] XMP integration:
  - [ ] XMP parsing
  - [ ] XMP box encoding
  - [ ] XMP preservation
- [ ] ICC profile handling:
  - [ ] ICC profile parsing (use existing libs)
  - [ ] ICC profile embedding
  - [ ] Color management integration
- [ ] Custom metadata boxes:
  - [ ] Generic box API
  - [ ] Custom box registration
  - [ ] Box preservation
- [ ] Tests and documentation

**Expected Outcomes:**
- Full metadata support
- Spec-compliant metadata boxes
- Metadata preservation

**Validation:**
- [ ] EXIF roundtrips correctly
- [ ] XMP roundtrips correctly
- [ ] ICC profiles embedded and used

---

### Phase 3 Deliverable: v1.0.0

**Target Metrics:**
- ✅ 100% spec compliance
- ✅ 0.15-0.18 BPP compression (matches libjxl)
- ✅ 5-15 MP/s encoding (comparable to libjxl)
- ✅ 100% conformance tests passing
- ✅ Production-ready for all use cases

**Release Criteria:**
- [ ] JPEG reconstruction mode working
- [ ] HDR support complete
- [ ] Full metadata support
- [ ] 100% conformance tests pass
- [ ] All features documented
- [ ] Performance optimized
- [ ] Production deployments validated
- [ ] v1.0.0 announcement and blog post

---

## Continuous Improvements (Ongoing)

### Performance Optimization

**Ongoing tasks:**
- [ ] Profile hot paths
- [ ] Add AVX-512 support
- [ ] Optimize memory allocations
- [ ] Cache-aware algorithms
- [ ] SIMD quantization (15% speedup expected)

### Documentation

**Ongoing tasks:**
- [ ] Keep docs in sync with implementation
- [ ] Add more examples
- [ ] Write blog posts about architecture
- [ ] Create video tutorials

### Testing

**Ongoing tasks:**
- [ ] Increase test coverage
- [ ] Add fuzzing
- [ ] Performance regression tests
- [ ] Conformance test expansion

### Community

**Ongoing tasks:**
- [ ] Respond to issues
- [ ] Review PRs
- [ ] Release management
- [ ] Community engagement

---

## Development Priorities

### Priority Legend

- ⭐⭐⭐⭐⭐ **Critical:** Blocks production readiness
- ⭐⭐⭐⭐ **High:** Significant impact on quality/performance
- ⭐⭐⭐ **Medium:** Important features for some use cases
- ⭐⭐ **Low-Medium:** Nice to have, spec compliance
- ⭐ **Low:** Enhancement, not critical

### Impact Legend

- 🔴 **CRITICAL:** Must have for production
- 🟠 **HIGH:** Major impact on usability/performance
- 🟡 **MEDIUM:** Significant for some use cases
- 🟢 **LOW-MEDIUM:** Important for completeness

---

## Timeline Summary

| Phase | Duration | Effort | Deliverable | Status |
|-------|----------|--------|-------------|--------|
| **Phase 1** | 1-2 months | 140-180 hours | v0.5.0 (85% compliance) | 🟡 Ready |
| **Phase 2** | 3-4 months | 220-340 hours | v0.9.0 (95% compliance) | 🔴 Waiting |
| **Phase 3** | 5-12 months | 220-300 hours | v1.0.0 (100% compliance) | 🔴 Waiting |
| **Total** | 9-18 months | 580-820 hours | Production-ready codec | 🟡 In progress |

**Assumptions:**
- 20 hours/week development pace
- Serial execution (one phase at a time)
- Parallel work possible with multiple contributors

---

## Success Criteria

### Technical Metrics

- [ ] 100% spec compliance
- [ ] 100% conformance tests passing
- [ ] Performance within 2× of libjxl
- [ ] Zero clippy warnings
- [ ] Zero compiler warnings
- [ ] >80% code coverage

### Quality Metrics

- [ ] Comprehensive documentation
- [ ] Clear API
- [ ] Good error messages
- [ ] Production deployments
- [ ] Community adoption

### Community Metrics

- [ ] Published to crates.io
- [ ] Blog posts and tutorials
- [ ] GitHub stars and forks
- [ ] Community contributions
- [ ] Production use cases

---

## Risk Management

### Technical Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| ANS complexity | High | Dedicate 80 hours, seek expert review |
| Performance targets | Medium | Profile early, optimize incrementally |
| Conformance failures | High | Test frequently, fix incrementally |
| API design changes | Medium | Stabilize API in Phase 1 |

### Schedule Risks

| Risk | Impact | Mitigation |
|------|--------|------------|
| Scope creep | High | Strict phase definitions, defer non-critical |
| Underestimation | Medium | Buffer time built into estimates |
| Dependency delays | Low | Minimal external dependencies |

---

## Next Steps (Immediate)

1. **Start Phase 1.1: Complete ANS Entropy Coding**
   - Debug complex distribution test
   - Implement context modeling
   - Target: 2× compression improvement

2. **Document current state:**
   - ✅ Update LIMITATIONS.md (done)
   - ✅ Update README.md (done)
   - ✅ Create ROADMAP.md (this document)
   - [ ] Update EVALUATION.md

3. **Set up development workflow:**
   - [ ] Create GitHub project board
   - [ ] Set up milestones (v0.5.0, v0.9.0, v1.0.0)
   - [ ] Create issue templates
   - [ ] Configure CI/CD for new features

4. **Community engagement:**
   - [ ] Announce updated roadmap
   - [ ] Seek contributors for Phase 1
   - [ ] Blog post about current state and goals

---

## Contributing

Want to help? See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

**High-impact contributions welcome:**
- ANS entropy coding completion (Phase 1.1)
- Conformance testing setup (Phase 1.2)
- Group-level parallelism (Phase 1.3)

**Contact:**
- Greg Lamberson: greg@lamco.io
- Repository: https://github.com/lamco-admin/jxl-rust-reference

---

**Last Updated:** November 13, 2025
**Next Review:** After Phase 1 completion (v0.5.0 release)
