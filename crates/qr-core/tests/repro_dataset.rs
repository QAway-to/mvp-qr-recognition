use qr_core::QRDecoder;
use std::path::PathBuf;

#[test]
fn test_dataset_rotation_failures() {
    // Enable logging to stdout with INFO level
    let _ = env_logger::builder()
        .filter_level(log::LevelFilter::Info)
        .is_test(true)
        .try_init();

    let root_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).parent().unwrap().parent().unwrap().to_path_buf();
    let dataset_dir = root_dir.join("generated_dataset");

    let targets = vec![
        "json_rot_30.png",
        "json_rot_45.png",
        "payment_rot_30.png",
        "payment_rot_45.png",
        "url_rot_30.png",
        "url_rot_45.png",
    ];

    let decoder = QRDecoder::new();

    for filename in targets {
        let file_path = dataset_dir.join(filename);
        println!("\n=== Testing {} ===", filename);
        
        if !file_path.exists() {
             println!("File not found: {:?}", file_path);
             continue;
        }

        let img = image::open(&file_path).expect("Failed to open image").to_luma8();
        
        match decoder.decode(&img) {
            Ok(res) => println!("✅ SUCCESS: {}", res.content),
            Err(e) => println!("❌ FAILED: {:?}", e),
        }
    }
}
