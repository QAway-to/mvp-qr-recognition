use image::GrayImage;
use qr_core::{OnnxDetector, QRDecoder};
use std::path::Path;

#[test]
fn debug_text_rot_45() {
    let img_path = Path::new("../../generated_dataset/text_rot_45.png");
    let model_path = Path::new("../../public/model.onnx");
    
    if !img_path.exists() || !model_path.exists() {
        eprintln!("Skipping: Files not found");
        return;
    }

    // 1. Load Image
    let img = image::open(img_path).expect("Open img").to_luma8();
    let (w, h) = img.dimensions();
    println!("Image loaded: {}x{}", w, h);

    // 2. Run ML Detection
    let model_bytes = std::fs::read(model_path).unwrap();
    let detector = OnnxDetector::load(&model_bytes).unwrap();
    
    // Verify hypothesis: Rotate image -45 degrees (maybe better alignment?)
    let rotated = qr_core::geometry::rotate_image(&img, -45.0);
    let _ = rotated.save("target/debug_rot/rotated_m45.png");
    println!("Rotated image saved to target/debug_rot/rotated_m45.png");

    // Detect on rotated with STANDARD threshold
    let detections = detector.detect(&rotated, Some(0.55)).expect("Detection failed");
    
    println!("Detections on rotated image: {}", detections.len());
    
    for (i, detect) in detections.iter().enumerate() {
        println!("Detection #{}: Conf={:.2} Box={:?}", i, detect.confidence, detect.bbox);
        let sent_img = &detect.image;
        let _ = sent_img.save(format!("target/debug_rot/rot_crop_{}.png", i));
        
        let decoder = QRDecoder::new();
        match decoder.decode(sent_img) {
            Ok(res) => println!("✅ Decoder success on rotated crop #{}: {}", i, res.content),
            Err(e) => println!("❌ Decoder failed on rotated crop #{}: {}", i, e),
        }
    }
}
