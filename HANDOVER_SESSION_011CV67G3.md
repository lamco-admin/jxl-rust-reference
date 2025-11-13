# JPEG XL Rust Implementation - Handover Document
## Session: claude/continue-work-and-read-011CV67G3sTBZCmVafvheWbe

**Date**: 2025-11-13
**Branch**: `claude/continue-work-and-read-011CV67G3sTBZCmVafvheWbe`
**Last Commit**: 345d5e9 "Add diagnostic tools for debugging rANS and AC coefficient issues"

---

## CRITICAL STATUS: WORK IN PROGRESS

**Current State**: 1 out of 5 integration tests passing (20%)

**Blocker**: rANS (Range Asymmetric Numeral System) implementation has a bug that breaks with large alphabets (>= 11 symbols), causing AC coefficients to be completely scrambled during encoding/decoding.

---

## TEST RESULTS (ACTUAL NUMBERS)

### Working:
- **Solid color test**: 34.21 dB ✅ (target: > 11 dB)
  - Works because solid colors only use DC coefficients (no AC)
  - AC coefficients all quantize to zero, so rANS bug is not triggered

### Failing:
- **8x8 gradient**: 8.37 dB ❌ (target: > 10 dB)
- **64x64 gradient**: 5.36 dB ❌ (target: > 11 dB)
- **32x32 gradient**: 5.70 dB ❌ (target: > 8 dB)
- **Different quality levels**: 6.36-6.49 dB ❌

All failures are due to AC coefficient scrambling from rANS bug.

---

## ROOT CAUSE ANALYSIS

### Bug Chain Identified:

1. **AC coefficients use large alphabet** (~270 symbols)
   - Coefficients range from -135 to +135
   - Zigzag encoding: 0→0, 1→1, -1→2, 2→3, -2→4, etc.
   - Symbol 269 = coefficient -135
   - Alphabet size = 270 symbols

2. **rANS encoder/decoder works for small alphabets** (< 11 symbols) ✅
   - Test with 5 symbols: PERFECT round-trip
   - Test with 8 symbols: PERFECT round-trip

3. **rANS BREAKS for alphabets >= 11 symbols** ❌
   - First 1-2 symbols decode correctly
   - Remaining symbols are completely wrong
   - Example with 11 symbols:
     ```
     Encoded: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10]
     Decoded: [0, 1, 5, 10, 1, ?, ?, ?, ?, ?, ?]  ← WRONG!
     ```

4. **Evidence from actual AC coefficient debugging**:
   ```
   Channel 1 ENCODED symbols: [125, 269, 36, 51, 52, 9, 4, 5, 15, 6]
   Channel 1 DECODED symbols: [125, 269,  1,  9,  4, 5, 4, 269, 24, 77]
                                         ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ WRONG!

   Notice:
   - Symbols 125, 269 correct (first 2)
   - Symbol 77 appears in decoded (came from DIFFERENT CHANNEL!)
   - Symbol 269 appears TWICE in decoded
   - Complete corruption after position 2
   ```

---

## BUGS FIXED (PARTIAL PROGRESS)

### Bug 1: rANS Frequency Normalization Overflow ✅ FIXED

**Location**: `crates/jxl-bitstream/src/ans.rs:57-130`

**Problem**:
```rust
// OLD CODE (BROKEN):
for (i, &freq) in frequencies.iter().enumerate() {
    if freq > 0 {
        let normalized = ((freq as u64 * ANS_TAB_SIZE as u64 + total as u64 / 2) / total as u64) as u32;
        normalized_freqs[i] = normalized.max(1); // ← BUG HERE!
        normalized_total += normalized_freqs[i];
    }
}
```

With 270 symbols at low frequency:
- Many symbols round to 0
- `.max(1)` bumps them all to 1
- Total exceeds 4096 (ANS_TAB_SIZE)
- Decode table indexing: `slot = cumul + freq` exceeds 4096 → panic!

**Fix Applied**:
- 3-pass normalization algorithm (lines 57-130)
- Pass 1: Compute normalized frequencies (no .max(1))
- Pass 2: Assign freq=1 to zero-freq symbols
- Pass 3: Redistribute to sum to exactly 4096
- Added assertions to verify correctness

**Verification**:
```rust
assert_eq!(cumul, ANS_TAB_SIZE); // Line 142
assert_eq!(slots_filled, ANS_TAB_SIZE as usize); // Line 161
```

**Result**: No more panics, distribution building verified correct. But decoding still fails!

---

## BUGS REMAINING (ACTIVE BLOCKER)

### Bug 2: rANS Encoding/Decoding Formula Bug ❌ IN PROGRESS

**Symptom**: First 1-2 symbols decode correctly, rest are wrong

**Location**: `crates/jxl-bitstream/src/ans.rs`
- Encoding: lines 168-197
- Decoding: lines 248-274

**Evidence from diagnostic tests**:

```bash
# Test results from tools/diagnose-gradient/examples/test_ans_256.rs:
Range [0..10]: 11 symbols → ✗ FAIL (Expected [0,1,2,3,4], Got [0,1,5,10,1])
Range [100..110]: 11 symbols → ✗ FAIL (Expected [100,101,102,103,104], Got [100,108,0,104,102])
Range [0..255]: 256 symbols → ✗ FAIL
Range [0..269]: 270 symbols → ✗ FAIL (Expected [0,1,2,3,4], Got [0,165,35,36,3])
```

**Distribution verified correct**:
- Frequency normalization sums to exactly 4096 ✅
- All 4096 decode table slots filled ✅
- Symbols with freq > 0 all have freq >= 1 ✅

**Encoding/decoding formulas currently used**:

```rust
// ENCODING (line 191-194):
let q = self.state / sym.freq;
let r = self.state % sym.freq;
self.state = (q << ANS_LOG_TAB_SIZE) + r + sym.cumul;

// DECODING (line 256-261):
let slot = (self.state & (ANS_TAB_SIZE - 1)) as usize;
let symbol = dist.decode_table[slot];
let sym = dist.symbols[symbol];
let quot = self.state >> ANS_LOG_TAB_SIZE;
let rem = self.state & (ANS_TAB_SIZE - 1);
self.state = sym.freq * quot + rem - sym.cumul;
```

**Hypothesis**: Bug is in renormalization (lines 186-189 encode, 264-271 decode) or state calculation. The fact that first 1-2 symbols work suggests the initial state is correct, but subsequent renormalization corrupts the stream.

---

## DIAGNOSTIC TOOLS CREATED

**Location**: `tools/diagnose-gradient/examples/`

All tools verified that every component EXCEPT rANS works correctly:

1. **test_dct.rs** - DCT/IDCT invertibility
   - Result: ✅ Max error < 0.0001
   - Conclusion: DCT/IDCT are correct

2. **test_zigzag_values.rs** - Zigzag scanning round-trip
   - Result: ✅ Perfect round-trip, all 256 values match
   - Conclusion: Zigzag is correct

3. **test_dc_ac_merge.rs** - DC/AC separation and merge
   - Result: ✅ Perfect round-trip
   - Conclusion: DC/AC handling is correct

4. **test_rans_directly.rs** - rANS with 5 symbols
   - Result: ✅ Perfect round-trip
   - Conclusion: rANS works for small alphabets

5. **test_large_alphabet_ans.rs** - rANS with 270 symbols
   - Result: ❌ First 2 symbols correct, rest wrong
   - Conclusion: **THIS IS THE BUG** - isolated to rANS with large alphabets

6. **test_ans_256.rs** - rANS at various alphabet sizes
   - Result: < 11 symbols ✅, >= 11 symbols ❌
   - Conclusion: Bug threshold is at 11 symbols

7. **test_multi_block.rs** - PSNR vs image size
   - Result: All gradients fail (5-8 dB)
   - Conclusion: Bug affects all non-solid images

8. **test_ans_ac.rs** - Full AC coefficient pipeline
   - Result: 7.28 dB, pixel errors up to 80
   - Conclusion: AC coefficients completely corrupted

**To run diagnostic**: `cargo run --manifest-path tools/diagnose-gradient/Cargo.toml --example <name>`

---

## CODE LOCATIONS

### Core rANS Implementation:
- **File**: `crates/jxl-bitstream/src/ans.rs`
- **Key functions**:
  - `AnsDistribution::from_frequencies()` - lines 36-175 (FIXED)
  - `RansEncoder::encode_symbol()` - lines 168-197 (BUG HERE)
  - `RansEncoder::finalize()` - lines 200-210
  - `RansDecoder::new()` - lines 228-246
  - `RansDecoder::decode_symbol()` - lines 248-274 (BUG HERE)

### AC Coefficient Encoding:
- **File**: `crates/jxl-encoder/src/lib.rs`
- **Function**: `encode_ac_coefficients_ans()` - lines 475-530
- **Debug logging**: Lines 510-512, 524

### AC Coefficient Decoding:
- **File**: `crates/jxl-decoder/src/lib.rs`
- **Function**: `decode_ac_coefficients_ans()` - lines 279-336
- **Debug logging**: Lines 306, 330-334

### Distribution Building:
- **File**: `crates/jxl-encoder/src/lib.rs`
- **Function**: `build_distribution()` - lines 369-410
- Builds shared AC distribution from all channels

---

## TECHNICAL DETAILS: rANS ALGORITHM

### Theory (for reference):

**rANS** is a LIFO (last-in-first-out) entropy coder:
- Symbols encoded in reverse order: [a,b,c] → encode(c), encode(b), encode(a)
- Decoder reads forward and gets [a,b,c] back
- State is maintained in range [ANS_TAB_SIZE, ∞)

**Encoding formula**: `C(s, x) = (x / f_s) * M + (x % f_s) + b_s`
- `x` = current state
- `s` = symbol to encode
- `f_s` = frequency of symbol s
- `b_s` = cumulative frequency before s (cumul)
- `M` = ANS_TAB_SIZE = 4096

**Decoding formula**: `D(x) = f_s * (x >> L) + (x & (M-1)) - b_s`
- `L` = ANS_LOG_TAB_SIZE = 12
- Symbol lookup: `s = decode_table[x & (M-1)]`

**Renormalization**:
- Encoding: While `x >= f_s * (M << 8)`, output `x & 0xFF`, shift `x >>= 8`
- Decoding: While `x < M`, read byte `b`, update `x = (x << 8) | b`

---

## CRITICAL CONSTANTS

```rust
pub const ANS_TAB_SIZE: u32 = 4096;  // M in formulas
const ANS_LOG_TAB_SIZE: u32 = 12;    // L in formulas, log2(4096)
```

**Why 4096?**
- Powers of 2 allow efficient bit operations
- Larger table = better compression but more memory
- 4096 is standard for tANS/rANS implementations

---

## XYB SCALING (WORKING CORRECTLY)

**Issue discovered**: XYB color space values are in range [0, 1], but DCT expects larger values for proper quantization.

**Fix applied** (lines 178-196 in encoder, 137-153 in decoder):
```rust
// Encoder: Scale by 255 BEFORE DCT
for val in &mut channel {
    *val *= 255.0;
}

// Decoder: Unscale by 255 AFTER IDCT
for val in &mut xyb_channel {
    *val /= 255.0;
}
```

**Result**: Solid colors work perfectly (34.21 dB), proving this fix is correct.

---

## COMMIT HISTORY (RECENT)

```
345d5e9 Add diagnostic tools for debugging rANS and AC coefficient issues
78782bd Fix critical rANS frequency normalization bug (partial fix)
df01729 Add diagnostic logging to trace critical AC coefficient bug
e822dde Add XYB scaling before DCT - partial fix for quantization issue
19c66f2 Fix CRITICAL u32 overflow bug in ANS renormalization
```

---

## NEXT STEPS (SPECIFIC ACTIONS)

### Immediate Priority: Fix rANS for Large Alphabets

**Step 1: Verify encoding formula is correct**

Compare against reference implementation or rANS paper. Check:
- Division/modulo vs multiplication in state calculation
- Shift amounts (should be ANS_LOG_TAB_SIZE = 12)
- Renormalization threshold calculation

**Step 2: Add detailed state tracing**

Add logging to both encoder and decoder to trace:
```rust
eprintln!("Symbol {}: state_before={}, freq={}, cumul={}, state_after={}",
          symbol, state_before, sym.freq, sym.cumul, state_after);
```

Run with 11-symbol test case and compare encoder vs decoder state sequences.

**Step 3: Test with known-good implementation**

If available, compare output against libjxl or another rANS implementation:
- Same input data
- Same distribution
- Compare encoded byte streams

**Step 4: Check for integer overflow/truncation**

With large alphabets:
- `sym.cumul` can be up to 4095
- `sym.freq` can vary widely
- Check if any intermediate calculations overflow u32
- Check if state exceeds safe range

**Step 5: Verify renormalization logic**

The renormalization is different for encoding vs decoding:
- Encoding: `while (state as u64) >= max_state` (line 186)
- Decoding: `while state < ANS_TAB_SIZE` (line 264)

With large alphabets, `max_state` calculation may need review:
```rust
let max_state = (sym.freq as u64) * ((ANS_TAB_SIZE << 8) as u64);
```

**Step 6: Test incremental fix**

Once a hypothesis is formed:
1. Apply fix to `crates/jxl-bitstream/src/ans.rs`
2. Run: `cargo run --manifest-path tools/diagnose-gradient/Cargo.toml --example test_ans_256`
3. Verify all ranges pass
4. Run: `cargo test --test roundtrip_test -- --nocapture`
5. Target: ALL tests should pass with PSNR > thresholds

---

## SUCCESS CRITERIA

**Definition of Done**:
1. All 5 integration tests pass:
   - `test_roundtrip_encode_decode`: PSNR > 11 dB
   - `test_roundtrip_different_sizes`: PSNR > 8 dB for all sizes
   - `test_roundtrip_different_quality_levels`: PSNR matches quality
   - `test_solid_color_image`: PSNR > 11 dB (already passes)
   - `test_ans_minimal_8x8_single_block`: PSNR > 10 dB

2. Diagnostic test `test_ans_256` shows all ranges pass:
   ```
   Range [0..10]: ✓ OK
   Range [100..110]: ✓ OK
   Range [250..260]: ✓ OK
   Range [0..255]: ✓ OK
   Range [0..256]: ✓ OK
   Range [0..269]: ✓ OK
   ```

3. Gradient images decode with correct PSNR (> 11 dB minimum)

---

## REFERENCE MATERIALS

### rANS Papers/Implementations:
- Original rANS paper: Duda, "Asymmetric numeral systems: entropy coding combining speed of Huffman coding with compression rate of arithmetic coding"
- libjxl implementation: `lib/jxl/ans_common.cc`, `lib/jxl/ans_params.h`
- Fabian Giesen's blog: "Interleaved Entropy Coders" (excellent rANS tutorial)

### JPEG XL Specification:
- Part 1, Section 7.3: ANS entropy coding
- Part 1, Annex A: ANS decoder algorithm

---

## WHAT WORKS (DO NOT MODIFY)

These components are verified correct through diagnostic tests:

✅ **DCT/IDCT transforms** - Invertible to < 0.0001 error
✅ **XYB color space conversion** - Has passing unit tests
✅ **XYB scaling** - Proven by solid color test (34 dB)
✅ **Zigzag scanning** - Perfect round-trip verified
✅ **DC/AC separation and merge** - Perfect round-trip verified
✅ **Quantization/dequantization** - Working (solid colors prove this)
✅ **DC coefficient encoding** - Working (solid colors prove this)
✅ **rANS with small alphabets** (< 11 symbols) - Perfect round-trip
✅ **rANS frequency normalization** - Fixed, verified with assertions

---

## WHAT'S BROKEN (FOCUS HERE)

❌ **rANS encoding/decoding with alphabets >= 11 symbols**
- Specifically: Symbol state calculation or renormalization
- Affects: AC coefficient encoding (uses ~270 symbol alphabet)
- Impact: 80% of integration tests fail with catastrophic PSNR

---

## DEBUG OUTPUT AVAILABLE

Remove debug logging after fix is confirmed:

**Encoder** (lines 510-512, 524):
```rust
eprintln!("DEBUG AC encode: first 10 coeffs = {:?}", ...);
eprintln!("DEBUG AC encode: first 10 symbols = {:?}", ...);
eprintln!("DEBUG AC encode: {} symbols -> {} ANS bytes", ...);
```

**Decoder** (lines 306, 330-334):
```rust
eprintln!("DEBUG AC decode: expecting {} positions, reading {} ANS bytes", ...);
eprintln!("DEBUG AC decode: first 10 coeffs = {:?}", ...);
eprintln!("DEBUG AC decode: first 10 symbols = {:?}", ...);
```

**To see output**: `cargo test --test roundtrip_test -- --nocapture 2>&1 | grep DEBUG`

---

## IMPORTANT NOTES

1. **NO SHORTCUTS**: Do not work around the rANS bug by limiting alphabet size or using a different entropy coder. Fix the actual bug in the rANS implementation.

2. **Verify with tests**: After any change, run both diagnostic tests AND integration tests to ensure the fix works end-to-end.

3. **Small alphabet case must still work**: Ensure that fixing large alphabets doesn't break the small alphabet case (< 11 symbols).

4. **Distribution building is correct**: The frequency normalization fix ensures distributions are valid. Do not modify `from_frequencies()` unless you find a specific bug with evidence.

5. **State tracing is key**: The bug manifests after the first 1-2 symbols, suggesting state corruption. Trace the state variable through encoding and decoding to find where it diverges.

---

## COMMANDS FOR NEXT SESSION

```bash
# Run all integration tests
cargo test --test roundtrip_test -- --nocapture

# Run specific diagnostic test
cargo run --manifest-path tools/diagnose-gradient/Cargo.toml --example test_ans_256

# Run large alphabet test
cargo run --manifest-path tools/diagnose-gradient/Cargo.toml --example test_large_alphabet_ans

# See debug output
cargo test --test roundtrip_test test_ans_minimal_8x8_single_block -- --nocapture 2>&1 | grep DEBUG

# Build without tests
cargo build --release
```

---

## FINAL STATUS

**Branch**: `claude/continue-work-and-read-011CV67G3sTBZCmVafvheWbe`
**Tests Passing**: 1/5 (20%)
**Blocker**: rANS large alphabet bug
**Priority**: CRITICAL - blocks all gradient/complex image encoding

**Estimated effort**: 2-4 hours to find and fix the rANS encoding/decoding formula bug, assuming systematic debugging with state tracing.

---

END OF HANDOVER DOCUMENT
