use image::GrayImage;
use qr_core::{QRScanner};
use std::path::Path;
use std::time::Instant;

#[test]
fn bench_generated_dataset() {
    // 0. Init Logger
    let _ = env_logger::builder().is_test(true).try_init();

    // 1. Setup Scanner
    let model_path = Path::new("../../public/model.onnx");
    if !model_path.exists() {
        eprintln!("SKIPPED: Model not found at {:?}", model_path);
        return;
    }

    let model_bytes = std::fs::read(model_path).expect("Failed to read model");
    
    use qr_core::OnnxDetector;
    let detector = OnnxDetector::load(&model_bytes).expect("Failed to load model");
    
    let mut scanner = QRScanner::new();
    scanner.set_ml_detector(detector);

    // 2. Load Dataset
    let dataset_dir = Path::new("../../generated_dataset");
    if !dataset_dir.exists() {
        eprintln!("SKIPPED: Dataset not found at {:?}", dataset_dir);
        return;
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(dataset_dir).expect("Failed to read dir") {
        let entry = entry.expect("Failed to read entry");
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "png" || ext == "jpg") {
            files.push(path);
        }
    }
    
    // Sort for consistent output
    files.sort();

    println!("Found {} images in generated_dataset", files.len());

    let mut success_count = 0;
    let mut total_dur = std::time::Duration::new(0, 0);

    for path in &files {
        let filename = path.file_name().unwrap().to_string_lossy();
        
        let img = image::open(path).expect("Failed to open image").to_luma8();
        
        let start = Instant::now();
        let results = scanner.scan_image(&img).expect("Scan failed").qr_codes;
        let duration = start.elapsed();
        
        total_dur += duration;

        if !results.is_empty() {
            success_count += 1;
            println!("✅ [OK] {} ({:.2}s) -> Found {} codes", filename, duration.as_secs_f32(), results.len());
            for res in results {
                println!("      Content: {}", res.content);
            }
        } else {
            println!("❌ [FAIL] {} ({:.2}s) - No QR found", filename, duration.as_secs_f32());
        }
    }
    
    let avg = total_dur.as_secs_f32() / files.len() as f32;
    println!("\n=== Final Benchmark Results ===");
    println!("Total: {}", files.len());
    println!("Success: {}/{} ({:.1}%)", success_count, files.len(), (success_count as f32 / files.len() as f32) * 100.0);
    println!("Average Time: {:.2}s/img", avg);
}
