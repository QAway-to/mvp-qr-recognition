use image::GrayImage;
use qr_core::{OnnxDetector, QRDecoder};
use std::path::Path;
use std::env;

#[test]
fn debug_rotation_experiment() {
    // 0. Configuration from Env
    let angle: f32 = env::var("ROT_ANGLE").unwrap_or("45.0".to_string()).parse().unwrap_or(45.0);
    let interp = env::var("ROT_INTERP").unwrap_or("bilinear".to_string()); // "bilinear", "nearest"
    let enhance = env::var("ROT_ENHANCE").unwrap_or("none".to_string()); // "none", "upscale", "threshold"
    let img_name = env::var("ROT_IMG").unwrap_or("text_rot_45.png".to_string());

    println!("=== EXPERIMENT CFG: Angle={}, Interp={}, Enhance={}, Image={} ===", angle, interp, enhance, img_name);

    let img_path_str = format!("../../generated_dataset/{}", img_name);
    let img_path = Path::new(&img_path_str);
    let model_path = Path::new("../../public/model.onnx");
    
    if !img_path.exists() || !model_path.exists() {
        eprintln!("Skipping: Files not found");
        return;
    }

    // 1. Load Image
    let mut img = image::open(img_path).expect("Open img").to_luma8();
    let (w, h) = img.dimensions();
    println!("Image loaded: {}x{}", w, h);

    // 2. Pre-Enhancement (Upscale)
    if enhance == "upscale" {
        use image::imageops::FilterType;
        println!("Applying Enhancement: Upscale 2x (Nearest)");
        // Use Nearest for upscale to keep edges sharp before rotation
        let upscaled = image::imageops::resize(&img, w * 2, h * 2, FilterType::Nearest);
        // Resize returns ImageBuffer, compatible with GrayImage
        img = upscaled;
    }

    // 3. Preparation: Rotate
    // Note: To support "nearest" vs "bilinear", we need to hack/modify where `rotate_image` calls `bilinear_sample`.
    // Since `geometry::rotate_image` is hardcoded in lib, we might need to duplicate logic here 
    // OR just updating `geometry.rs` temporarily.
    // OPTION: We will use the `geometry::rotate_image` BUT since we can't easily change its internal interp 
    // from here without editing src code, we will rely on `ROT_INTERP` being handled by editing `geometry.rs` 
    // OR we just implement a local rotate here for the test to prove the point.
    // Let's implement a LOCAL rotate helper here to verify the "Nearest" hypothesis efficiently without 
    // constantly editing lib code.
    
    let rotated = local_rotate(&img, angle, &interp);
    
    let output_dir_str = env::var("ROT_OUTPUT_DIR").unwrap_or("target/debug_rot".to_string());
    let output_dir = Path::new(&output_dir_str);
    let _ = std::fs::create_dir_all(output_dir);
    
    let _ = rotated.save(output_dir.join("experiment_rotated.png"));

    // 4. Run ML Detection
    let model_bytes = std::fs::read(model_path).unwrap();
    let detector = OnnxDetector::load(&model_bytes).unwrap();
    
    let thresh: f32 = env::var("ROT_THRESH").unwrap_or("0.5".to_string()).parse().unwrap_or(0.5);
    
    // Use permissive threshold to ensure we get crops
    let detections = detector.detect(&rotated, Some(thresh)).expect("Detection failed");
    println!("Detections found: {}", detections.len());
    
    // Limit to top 5 detections to safely ignore noise storms
    for (i, detect) in detections.iter().take(5).enumerate() {
        let mut crop = detect.image.clone();
        
        // 5. Post-Enhancement (Threshold)
        if enhance == "threshold" {
             println!("Applying Enhancement: Threshold (128)");
             for p in crop.pixels_mut() {
                p.0[0] = if p.0[0] < 128 { 0 } else { 255 };
             }
        }
        
        let _ = crop.save(output_dir.join(format!("crop_{}.png", i)));
        
        // 6. Decode
        let decoder = QRDecoder::new();
        match decoder.decode(&crop) {
            Ok(res) => println!("✅ Decoder success on crop #{}: {}", i, res.content),
            Err(e) => println!("❌ Decoder failed on crop #{}: {}", i, e),
        }
    }
}

// Local helper to test interpolation difference
fn local_rotate(img: &GrayImage, angle_deg: f32, func: &str) -> GrayImage {
    use image::Luma;
    let (w, h) = img.dimensions();
    let angle_rad = angle_deg.to_radians();
    let cos_a = angle_rad.cos();
    let sin_a = angle_rad.sin();
    
    let (cx, cy) = (w as f32 / 2.0, h as f32 / 2.0);
    
    // Bounds calc (simplified, keep original size or expand? expanding is safer for corners)
    // For this test, let's keep it simple and expand to fit.
    let abs_cos = cos_a.abs();
    let abs_sin = sin_a.abs();
    let new_w = (w as f32 * abs_cos + h as f32 * abs_sin).ceil() as u32;
    let new_h = (w as f32 * abs_sin + h as f32 * abs_cos).ceil() as u32;
    
    let mut new_img = GrayImage::from_pixel(new_w, new_h, Luma([255]));
    let (ncx, ncy) = (new_w as f32 / 2.0, new_h as f32 / 2.0);

    for y in 0..new_h {
        for x in 0..new_w {
            let dx = x as f32 - ncx;
            let dy = y as f32 - ncy;
            
            let src_x = dx * cos_a + dy * sin_a + cx;
            let src_y = -dx * sin_a + dy * cos_a + cy;
            
            if src_x >= 0.0 && src_x < (w as f32 - 1.0) && src_y >= 0.0 && src_y < (h as f32 - 1.0) {
                 let pixel_val = if func == "nearest" {
                     let sx = src_x.round() as u32;
                     let sy = src_y.round() as u32;
                     if sx < w && sy < h {
                        img.get_pixel(sx, sy).0[0]
                     } else { 255 }
                 } else {
                     // Bilinear (simplified implementation or use geometry::bilinear if public?)
                     // Let's implement simple bilinear here to be self-contained
                     let fx = src_x.floor();
                     let fy = src_y.floor();
                     let tx = src_x - fx;
                     let ty = src_y - fy;
                     let ix = fx as u32;
                     let iy = fy as u32;
                     
                     let p00 = img.get_pixel(ix, iy).0[0] as f32;
                     let p10 = img.get_pixel(ix + 1, iy).0[0] as f32;
                     let p01 = img.get_pixel(ix, iy + 1).0[0] as f32;
                     let p11 = img.get_pixel(ix + 1, iy + 1).0[0] as f32;
                     
                     let top = p00 * (1.0 - tx) + p10 * tx;
                     let bot = p01 * (1.0 - tx) + p11 * tx;
                     (top * (1.0 - ty) + bot * ty) as u8
                 };
                 new_img.put_pixel(x, y, Luma([pixel_val]));
            }
        }
    }
    new_img
}
