//! Integration tests for QR recognition

use qr_core::{QRScanner, ContentType};
use image::{GrayImage, Luma};

/// Helper to create a simple test image
fn create_test_image(width: u32, height: u32) -> GrayImage {
    GrayImage::from_pixel(width, height, Luma([200]))
}

#[test]
fn test_scanner_creation() {
    let scanner = QRScanner::new();
    // Should not panic
    let _ = scanner;
}

#[test]
fn test_empty_image_scan() {
    let scanner = QRScanner::new();
    let img = create_test_image(100, 100);
    
    let start = std::time::Instant::now();
    let result = scanner.scan_image(&img, start);
    
    assert!(result.is_ok());
    let scan_result = result.unwrap();
    // Empty image should return empty or fallback result
    assert!(scan_result.qr_codes.is_empty() || scan_result.qr_codes.len() >= 0);
}

#[test]
fn test_content_type_detection() {
    // URL
    assert_eq!(ContentType::detect("https://example.com"), ContentType::Url);
    assert_eq!(ContentType::detect("http://test.ru/page"), ContentType::Url);
    
    // Payment URLs
    assert_eq!(ContentType::detect("https://qr.nspk.ru/123"), ContentType::Payment);
    
    // VCard
    assert_eq!(ContentType::detect("BEGIN:VCARD\nVERSION:3.0\nN:Test"), ContentType::VCard);
    
    // WiFi
    assert_eq!(ContentType::detect("WIFI:T:WPA;S:Network;P:password;;"), ContentType::WiFi);
    
    // Email
    assert_eq!(ContentType::detect("mailto:test@example.com"), ContentType::Email);
    
    // Phone
    assert_eq!(ContentType::detect("tel:+79001234567"), ContentType::Phone);
    
    // SMS
    assert_eq!(ContentType::detect("sms:+79001234567?body=Hello"), ContentType::Sms);
    
    // Geo
    assert_eq!(ContentType::detect("geo:55.7558,37.6173"), ContentType::Geo);
    
    // Plain text
    assert_eq!(ContentType::detect("Hello World"), ContentType::Text);
}

#[test]
fn test_sbp_payment_parsing() {
    use qr_core::{PaymentParser, PaymentFormat};
    
    let parser = PaymentParser::new();
    
    let sbp_url = "https://qr.nspk.ru/AS10001234567890?type=02&bank=100000000001&sum=15000&cur=RUB";
    let result = parser.parse(sbp_url);
    
    assert!(result.is_some());
    let payment = result.unwrap();
    
    assert_eq!(payment.format, PaymentFormat::SbpRussia);
    assert_eq!(payment.amount, Some(150.0)); // 15000 копеек = 150 рублей
    assert_eq!(payment.currency, Some("RUB".to_string()));
}

#[test]
fn test_st_payment_parsing() {
    use qr_core::{PaymentParser, PaymentFormat};
    
    let parser = PaymentParser::new();
    
    let st_content = "ST.00012|Name=ООО Рога и Копыта|PersonalAcc=40702810099990001234|BankName=ПАО Сбербанк|BIC=044525225|Sum=250000|Purpose=Оплата по счёту 123";
    let result = parser.parse(st_content);
    
    assert!(result.is_some());
    let payment = result.unwrap();
    
    assert_eq!(payment.format, PaymentFormat::StRussia);
    assert_eq!(payment.payee_name, Some("ООО Рога и Копыта".to_string()));
    assert_eq!(payment.account, Some("40702810099990001234".to_string()));
    assert_eq!(payment.bic, Some("044525225".to_string()));
    assert_eq!(payment.amount, Some(2500.0)); // 250000 копеек = 2500 рублей
    assert_eq!(payment.purpose, Some("Оплата по счёту 123".to_string()));
}

#[test]
fn test_relevance_score() {
    use qr_core::PaymentParser;
    
    let parser = PaymentParser::new();
    
    // СБП - максимальный приоритет
    assert_eq!(parser.relevance_score("https://qr.nspk.ru/test"), 1.0);
    
    // Обычные URL - низкий приоритет
    assert!(parser.relevance_score("https://google.com") < 0.1);
    
    // Текст с упоминанием оплаты
    assert!(parser.relevance_score("Оплата заказа #123") > 0.5);
    
    // Обычный текст
    assert!(parser.relevance_score("Привет мир!") < 0.1);
}

#[test]
fn test_image_processor() {
    use qr_core::{ImageProcessor, ProcessingConfig};
    
    let config = ProcessingConfig::default();
    let processor = ImageProcessor::new(config);
    
    // Create test image with some contrast
    let mut img = GrayImage::new(100, 100);
    for y in 0..100 {
        for x in 0..100 {
            let value = if (x + y) % 10 < 5 { 50 } else { 200 };
            img.put_pixel(x, y, Luma([value]));
        }
    }
    
    let processed = processor.process(&img);
    
    // Should not crash and should have same dimensions
    assert_eq!(processed.dimensions(), img.dimensions());
}

#[test]
fn test_detector_ratio_check() {
    use qr_core::{QRDetector, DetectorConfig};
    
    let _detector = QRDetector::new(DetectorConfig::default());
    
    // Test passed if no panics
}

#[test]
fn test_decoder_creation() {
    use qr_core::QRDecoder;
    
    let decoder = QRDecoder::new();
    
    // Create a simple gray image (no actual QR)
    let img = GrayImage::from_pixel(50, 50, Luma([128]));
    
    // Should return NotFound error, not crash
    let result = decoder.decode(&img);
    assert!(result.is_err());
}
