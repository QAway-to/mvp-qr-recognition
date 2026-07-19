#[cfg(feature = "ml")]
#[test]
fn test_full_pipeline_random_sample() {
    use qr_core::{QRScanner, OnnxDetector};
    use std::path::PathBuf;
    use std::fs;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    // Enable logging
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    let dataset_dir = root_dir.join("training/yolo-2/test/images");
    let model_path = root_dir.join("public/model.onnx");

    println!("\n=== Full Pipeline Benchmark (Random 50) ===");
    println!("Dataset: {:?}", dataset_dir);
    println!("Model: {:?}", model_path);

    if !dataset_dir.exists() {
        println!("⚠️ Dataset not found. Skipping benchmark.");
        return;
    }
    if !model_path.exists() {
        panic!("Model file not found!");
    }

    // 1. Initialize Scanner with ML
    let model_bytes = fs::read(&model_path).expect("Failed to read model");
    let detector = OnnxDetector::load(&model_bytes).expect("Failed to load OnnxDetector");
    
    let mut scanner = QRScanner::new();
    scanner.set_ml_detector(detector);

    // 2. Select Images
    let entries: Vec<_> = fs::read_dir(&dataset_dir)
        .expect("Failed to read dataset dir")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            ext == "jpg" || ext == "jpeg" || ext == "png"
        })
        .collect();

    println!("Total images available: {}", entries.len());

    let mut rng = thread_rng();
    let sample = entries.choose_multiple(&mut rng, 50).collect::<Vec<_>>();

    println!("Selected {} images for validation.", sample.len());

    // 3. Run Benchmark
    let mut detected_count = 0;
    let mut decoded_count = 0;
    
    for (i, path) in sample.iter().enumerate() {
        let filename = path.file_name().unwrap().to_string_lossy();
        // println!("\n[Image {}/{}] {}", i+1, sample.len(), filename);
        
        let img = image::open(path).unwrap().to_luma8();
        
        // Measure time per image if needed, but here we care about accuracy
        let start = std::time::Instant::now();
        let result = scanner.scan_image(&img);
        let duration = start.elapsed();

        match result {
            Ok(res) => {
                if !res.qr_codes.is_empty() {
                    detected_count += 1;
                    
                    // Check if content is valid (not empty)
                    let content = &res.qr_codes[0].content;
                    if !content.is_empty() {
                        decoded_count += 1;
                        println!("✅ [OK] {} ({:.2?}) -> {}", filename, duration, content);
                    } else {
                        println!("⚠️ [DETECTED-EMPTY] {} ({:.2?})", filename, duration);
                    }
                } else {
                     println!("❌ [FAIL] {} ({:.2?}) - No QR found", filename, duration);
                }
            }
            Err(e) => {
                 println!("❌ [ERR] {} ({:.2?}) - {:?}", filename, duration, e);
            }
        }
    }

    println!("\n=== RESULTS ===");
    println!("Total: {}", sample.len());
    println!("Detected: {} ({:.1}%)", detected_count, (detected_count as f32 / sample.len() as f32) * 100.0);
    println!("Decoded: {} ({:.1}%)", decoded_count, (decoded_count as f32 / sample.len() as f32) * 100.0);
    
    // Assert somewhat reasonable performance for 'Hard' dataset (expect > 70%)
    // assert!(decoded_count as f32 / sample.len() as f32 > 0.5, "Success rate should be at least 50% on hard dataset with INT8 model");
}
