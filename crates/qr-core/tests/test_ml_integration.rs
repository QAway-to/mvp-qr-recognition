use qr_core::{OnnxDetector, QRDecoder};
use std::path::PathBuf;
use std::fs;

// Only compile/run this test if the "ml" feature is enabled
#[cfg(feature = "ml")]
#[test]
fn test_ml_detection_on_real_image() {
    // Enable detailed logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    let model_path = root_dir.join("public/model.onnx");
    let image_path = PathBuf::from("C:/Users/sadov/.gemini/antigravity/brain/bca0fb10-e329-4b4d-af98-8e8b0cd56a93/uploaded_image_1767560500046.jpg");

    println!("\n=== Testing ML Detection ===");
    println!("Model: {:?}", model_path);
    println!("Image: {:?}", image_path);

    if !model_path.exists() {
        panic!("Model file not found at {:?}", model_path);
    }
    if !image_path.exists() {
         panic!("Image file not found at {:?}", image_path);
    }

    // 1. Load Model
    let model_bytes = fs::read(&model_path).expect("Failed to read model");
    let detector = OnnxDetector::load(&model_bytes).expect("Failed to load OnnxDetector");

    // 2. Load Image
    let img = image::open(&image_path).expect("Failed to open image").to_luma8();
    println!("Image size: {:?}", img.dimensions());

    // 3. Run Detection
    let detections = detector.detect(&img).expect("Detection failed");
    
    println!("Found {} detections", detections.len());

    let decoder = QRDecoder::new();

    for (i, d) in detections.iter().enumerate() {
        println!("--- Detection #{} (Conf: {:.2}) ---", i, d.confidence);
        println!("BBox: {:?}", d.bbox);
        
        // Save cropped image for inspection - CONFIRM THRESHOLDING
        let debug_path = format!("debug_crop_threshold_{}.png", i);
        d.image.save(&debug_path).unwrap();
        println!("Saved thresholded crop to: {}", debug_path);

        // Verify it is binary
        let is_binary = d.image.pixels().all(|p| p.0[0] == 0 || p.0[0] == 255);
        if is_binary {
            println!("✅ Image is binary (Adaptive Threshold applied)");
        } else {
            println!("❌ Image is NOT binary (Adaptive Threshold failed?)");
        }

        // 4. Try to Decode the crop
        match decoder.decode(&d.image) {
            Ok(res) => println!("✅ DECODE SUCCESS: {}", res.content),
            Err(e) => println!("❌ DECODE FAILED: {:?}", e),
        }
    }

    if detections.is_empty() {
        println!("❌ NO QR CODES DETECTED BY ML MODEL");
    }
}
