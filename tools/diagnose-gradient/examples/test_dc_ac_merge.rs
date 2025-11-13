// Test if separate_dc_ac and merge_dc_ac preserve data

use jxl_transform::{zigzag_scan_channel, separate_dc_ac, merge_dc_ac, inv_zigzag_scan_channel};

fn main() {
    println!("Testing DC/AC separation and merging...\n");

    // Create test data: 16x16 image = 4 blocks
    let width = 16;
    let height = 16;
    let coeffs: Vec<i16> = (0..(width * height)).map(|i| (i % 100) as i16).collect();

    println!("Original coeffs (first 32): {:?}", &coeffs[0..32]);

    // Zigzag scan
    let mut zigzag_data = Vec::new();
    zigzag_scan_channel(&coeffs, width, height, &mut zigzag_data);

    println!("Zigzag data len: {}, should be: {}", zigzag_data.len(), 4 * 64);

    // Separate DC and AC
    let (dc, ac) = separate_dc_ac(&zigzag_data);
    println!("DC len: {} (should be 4 blocks)", dc.len());
    println!("AC len: {} (should be 4*63=252)", ac.len());
    println!("DC values: {:?}", dc);

    // Merge back
    let mut merged = Vec::new();
    merge_dc_ac(&dc, &ac, &mut merged);

    println!("Merged len: {}, should be: {}", merged.len(), zigzag_data.len());

    // Check if they match
    if zigzag_data.len() != merged.len() {
        println!("\n✗ LENGTH MISMATCH! {} != {}", zigzag_data.len(), merged.len());
    } else {
        let mut mismatches = 0;
        for i in 0..zigzag_data.len() {
            if zigzag_data[i] != merged[i] {
                if mismatches < 10 {
                    println!("Position {}: {} != {}", i, zigzag_data[i], merged[i]);
                }
                mismatches += 1;
            }
        }

        if mismatches == 0 {
            println!("\n✓ Perfect DC/AC round-trip!");
        } else {
            println!("\n✗ {} MISMATCHES!", mismatches);
        }
    }

    // Now test inv_zigzag to get back to spatial
    let mut reconstructed = Vec::new();
    inv_zigzag_scan_channel(&merged, width, height, &mut reconstructed);

    println!("\nFinal reconstruction check:");
    let mut final_mismatches = 0;
    for i in 0..coeffs.len() {
        if coeffs[i] != reconstructed[i] {
            if final_mismatches < 10 {
                println!("Position {}: {} != {}", i, coeffs[i], reconstructed[i]);
            }
            final_mismatches += 1;
        }
    }

    if final_mismatches == 0 {
        println!("✓ Full pipeline preserves all {} values!", coeffs.len());
    } else {
        println!("✗ {} values corrupted in full pipeline!", final_mismatches);
    }
}
