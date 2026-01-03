//! Test QR image generator
//! 
//! This tool generates test QR codes for testing the scanner.
//! Run with: cargo run -p qr-test-gen

use image::{GrayImage, Luma, Rgb, RgbImage};
use std::path::Path;

fn main() {
    let output_dir = Path::new("tests/images");
    std::fs::create_dir_all(output_dir).expect("Failed to create output directory");
    
    println!("Generating test QR images...");
    
    // Generate various test patterns
    generate_finder_pattern_test(output_dir);
    generate_gradient_test(output_dir);
    generate_noise_test(output_dir);
    generate_low_contrast_test(output_dir);
    
    println!("Done! Test images saved to tests/images/");
}

fn generate_finder_pattern_test(output_dir: &Path) {
    // Create image with finder pattern
    let size = 200u32;
    let mut img = GrayImage::from_pixel(size, size, Luma([255]));
    
    // Draw a simple finder pattern (7x7 modules, each module = 10px)
    let module_size = 10;
    let pattern_size = 7 * module_size;
    let start_x = 20;
    let start_y = 20;
    
    // Outer black square
    for y in 0..pattern_size {
        for x in 0..pattern_size {
            let row = y / module_size;
            let col = x / module_size;
            
            // Pattern: 1:1:3:1:1
            let is_black = match (row, col) {
                (0, _) | (6, _) | (_, 0) | (_, 6) => true, // Outer border
                (1, 1..=5) | (5, 1..=5) | (1..=5, 1) | (1..=5, 5) => false, // White ring
                (2..=4, 2..=4) => true, // Inner black square
                _ => false,
            };
            
            if is_black {
                img.put_pixel(start_x + x, start_y + y, Luma([0]));
            }
        }
    }
    
    img.save(output_dir.join("finder_pattern.png")).expect("Failed to save");
    println!("  Created finder_pattern.png");
}

fn generate_gradient_test(output_dir: &Path) {
    // Image with gradient background (simulates uneven lighting)
    let size = 300u32;
    let mut img = GrayImage::new(size, size);
    
    for y in 0..size {
        for x in 0..size {
            let gradient = ((x as f32 / size as f32) * 100.0 + 50.0) as u8;
            img.put_pixel(x, y, Luma([gradient]));
        }
    }
    
    img.save(output_dir.join("gradient_background.png")).expect("Failed to save");
    println!("  Created gradient_background.png");
}

fn generate_noise_test(output_dir: &Path) {
    // Image with noise
    let size = 200u32;
    let mut img = GrayImage::new(size, size);
    
    for y in 0..size {
        for x in 0..size {
            // Simple pseudo-random noise
            let noise = ((x * 17 + y * 31 + x * y) % 50) as u8;
            let base = if ((x / 20) + (y / 20)) % 2 == 0 { 30 } else { 220 };
            img.put_pixel(x, y, Luma([base.saturating_add(noise).saturating_sub(25)]));
        }
    }
    
    img.save(output_dir.join("noisy_pattern.png")).expect("Failed to save");
    println!("  Created noisy_pattern.png");
}

fn generate_low_contrast_test(output_dir: &Path) {
    // Low contrast image
    let size = 200u32;
    let mut img = GrayImage::new(size, size);
    
    for y in 0..size {
        for x in 0..size {
            let value = if ((x / 20) + (y / 20)) % 2 == 0 { 100 } else { 150 };
            img.put_pixel(x, y, Luma([value]));
        }
    }
    
    img.save(output_dir.join("low_contrast.png")).expect("Failed to save");
    println!("  Created low_contrast.png");
}
