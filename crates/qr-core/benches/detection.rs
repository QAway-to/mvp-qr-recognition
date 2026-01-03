//! Benchmarks for QR detection performance

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use image::{GrayImage, Luma};
use qr_core::{ImageProcessor, ProcessingConfig, QRDetector, DetectorConfig};

fn create_test_image(size: u32) -> GrayImage {
    let mut img = GrayImage::new(size, size);
    
    // Create some patterns to make detection interesting
    for y in 0..size {
        for x in 0..size {
            let value = if ((x / 10) + (y / 10)) % 2 == 0 { 0 } else { 255 };
            img.put_pixel(x, y, Luma([value]));
        }
    }
    
    img
}

fn benchmark_preprocessing(c: &mut Criterion) {
    let processor = ImageProcessor::new(ProcessingConfig::default());
    let img_small = create_test_image(320);
    let img_medium = create_test_image(640);
    let img_large = create_test_image(1280);
    
    c.bench_function("preprocess_320x320", |b| {
        b.iter(|| processor.process(black_box(&img_small)))
    });
    
    c.bench_function("preprocess_640x640", |b| {
        b.iter(|| processor.process(black_box(&img_medium)))
    });
    
    c.bench_function("preprocess_1280x1280", |b| {
        b.iter(|| processor.process(black_box(&img_large)))
    });
}

fn benchmark_detection(c: &mut Criterion) {
    let detector = QRDetector::new(DetectorConfig::default());
    let img_small = create_test_image(320);
    let img_medium = create_test_image(640);
    
    c.bench_function("detect_320x320", |b| {
        b.iter(|| detector.detect(black_box(&img_small)))
    });
    
    c.bench_function("detect_640x640", |b| {
        b.iter(|| detector.detect(black_box(&img_medium)))
    });
}

criterion_group!(benches, benchmark_preprocessing, benchmark_detection);
criterion_main!(benches);
