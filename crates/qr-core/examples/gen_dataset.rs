//! Generator of synthetic QR code dataset
//! 
//! Usage: cargo run -p qr-core --example gen_dataset

use image::{GrayImage, Luma};
use imageproc::filter::gaussian_blur_f32;
use imageproc::geometric_transformations::{rotate_about_center, Interpolation};
use qrcode::QrCode;
use rand::Rng;
use std::path::Path;
use std::fs;

fn main() {
    let output_dir = Path::new("generated_dataset");
    if output_dir.exists() {
        fs::remove_dir_all(output_dir).unwrap();
    }
    fs::create_dir_all(output_dir).unwrap();

    println!("Generating dataset in {:?}", output_dir);

    let payloads = vec![
        ("payment", "https://qr.nspk.ru/AS10003P3D0G21577HMN0D5030303030?type=01&bank=100000000008&crc=0000"),
        ("url", "https://github.com/QAway-to/mvp-qr-recognition"),
        ("text", "This is a test QR code for WASM scanner verification."),
        ("json", "{\"id\":123,\"name\":\"Test Item\",\"active\":true}"),
    ];

    let mut count = 0;

    for (cat, content) in &payloads {
        // 1. Clean images
        let qr = QrCode::new(content).unwrap();
        
        // Manual render to avoid Image crate version mismatch (qrcode uses old image)
        let module_size = 10u32;
        let quiet_zone = 4u32;
        let width = qr.width() as u32;
        let doc_width = (width + quiet_zone * 2) * module_size;
        let mut img = GrayImage::from_pixel(doc_width, doc_width, Luma([255]));

        for y in 0..width {
            for x in 0..width {
                if qr[(x as usize, y as usize)] == qrcode::Color::Dark {
                    let px = (quiet_zone + x) * module_size;
                    let py = (quiet_zone + y) * module_size;
                    for dy in 0..module_size {
                        for dx in 0..module_size {
                            img.put_pixel(px + dx, py + dy, Luma([0]));
                        }
                    }
                }
            }
        }

        save(&img, output_dir, &format!("{}_clean.png", cat));
        count += 1;

        // 2. Blurred
        let blurred = gaussian_blur_f32(&img, 2.0);
        save(&blurred, output_dir, &format!("{}_blur_2.0.png", cat));
        count += 1;

        // 3. Rotated
        for angle in [15.0f32, 30.0, 45.0, 90.0] {
            let rotated = rotate_about_center(
                &img, 
                angle.to_radians(), 
                Interpolation::Bilinear, 
                Luma([255])
            );
            save(&rotated, output_dir, &format!("{}_rot_{}.png", cat, angle));
            count += 1;
        }

        // 4. Noisy (Salt & Pepper)
        let mut noisy = img.clone();
        for p in noisy.pixels_mut() {
            let r: f64 = rand::thread_rng().gen();
            if r < 0.05 {
                p.0[0] = if rand::thread_rng().gen() { 0 } else { 255 };
            }
        }
        save(&noisy, output_dir, &format!("{}_noise.png", cat));
        count += 1;

        // 5. Low Contrast
        let mut low_contrast = img.clone();
        for p in low_contrast.pixels_mut() {
            // Map 0..255 to 100..150
            let val = p.0[0] as f32;
            let new_val = 100.0 + (val / 255.0) * 50.0;
            p.0[0] = new_val as u8;
        }
        save(&low_contrast, output_dir, &format!("{}_low_contrast.png", cat));
        count += 1;
    }

    println!("Generated {} images.", count);
}

fn save(img: &GrayImage, dir: &Path, name: &str) {
    img.save(dir.join(name)).unwrap();
}
