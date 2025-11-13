use jxl_transform::{zigzag_scan_channel, inv_zigzag_scan_channel};

fn main() {
    println!("Testing zigzag round-trip with value verification...\n");

    let width = 16;
    let height = 16;
    let coeffs: Vec<i16> = (0..(width * height)).map(|i| i as i16).collect();

    println!("Original coeffs (first 32): {:?}", &coeffs[0..32]);

    let mut zigzag_data = Vec::new();
    zigzag_scan_channel(&coeffs, width, height, &mut zigzag_data);

    println!("Zigzag data (first 32): {:?}", &zigzag_data[0..32]);

    let mut reconstructed = Vec::new();
    inv_zigzag_scan_channel(&zigzag_data, width, height, &mut reconstructed);

    println!("Reconstructed (first 32): {:?}", &reconstructed[0..32]);

    // Check if they match
    let mut mismatches = 0;
    for i in 0..coeffs.len() {
        if coeffs[i] != reconstructed[i] {
            if mismatches < 10 {
                println!("Mismatch at position {}: {} != {}", i, coeffs[i], reconstructed[i]);
            }
            mismatches += 1;
        }
    }

    if mismatches == 0 {
        println!("\n✓ Perfect round-trip! All {} values match.", coeffs.len());
    } else {
        println!("\n✗ FAILURE! {} out of {} values don't match!", mismatches, coeffs.len());
    }
}
