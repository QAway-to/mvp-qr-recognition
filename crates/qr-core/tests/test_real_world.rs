use qr_core::QRDecoder;
use std::path::PathBuf;

#[test]
fn test_real_world_image() {
    // Enable logging to stdout with INFO level
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    // Specific path to the uploaded image
    let file_path = PathBuf::from("C:/Users/sadov/.gemini/antigravity/brain/bca0fb10-e329-4b4d-af98-8e8b0cd56a93/uploaded_image_1767560500046.jpg");
    
    println!("\n=== Testing Real World Image: {:?} ===", file_path);
    
    if !file_path.exists() {
         println!("File not found: {:?}", file_path);
         return;
    }

    let decoder = QRDecoder::new();
    let img = image::open(&file_path).expect("Failed to open image").to_luma8();
    
    match decoder.decode(&img) {
        Ok(res) => println!("✅ SUCCESS: {}", res.content),
        Err(e) => println!("❌ FAILED: {:?}", e),
    }
}
