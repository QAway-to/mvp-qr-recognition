//! QR Core - Модуль распознавания QR-кодов
//! 
//! Библиотека для обнаружения и декодирования QR-кодов с поддержкой:
//! - Предобработки изображений (коррекция перспективы, шумоподавление, контраст)
//! - Обнаружения множественных QR-кодов
//! - Декодирования через rxing с fallback на rqrr
//! - Парсинга платёжных форматов (EMV, СБП)

pub mod preprocessing;
pub mod detection;
pub mod decoding;
pub mod payment;

pub use preprocessing::{ImageProcessor, ProcessingConfig};
pub use detection::{QRDetector, DetectedQR, DetectorConfig};
pub use decoding::{QRDecoder, DecodedQR, DecodeError};
pub use payment::{PaymentParser, PaymentInfo, PaymentFormat};

use image::GrayImage;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Основные ошибки модуля
#[derive(Error, Debug)]
pub enum QRError {
    #[error("Image processing error: {0}")]
    ImageProcessing(String),
    
    #[error("Detection error: {0}")]
    Detection(String),
    
    #[error("Decode error: {0}")]
    Decode(#[from] DecodeError),
    
    #[error("Invalid image format: {0}")]
    InvalidFormat(String),
}

/// Результат полного сканирования
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    /// Все обнаруженные и декодированные QR-коды
    pub qr_codes: Vec<QRResult>,
    /// Наиболее релевантный платёжный QR (если есть)
    pub best_payment: Option<usize>,
    /// Время обработки в миллисекундах
    pub processing_time_ms: u64,
}

/// Результат для одного QR-кода
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QRResult {
    /// Декодированный контент
    pub content: String,
    /// Bounding box [x, y, width, height]
    pub bbox: [u32; 4],
    /// Тип контента
    pub content_type: ContentType,
    /// Платёжная информация (если это платёжный QR)
    pub payment: Option<PaymentInfo>,
    /// Уверенность детекции (0.0 - 1.0)
    pub confidence: f32,
}

/// Тип контента QR-кода
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContentType {
    Text,
    Url,
    VCard,
    WiFi,
    Payment,
    Email,
    Phone,
    Sms,
    Geo,
    Unknown,
}

impl ContentType {
    pub fn detect(content: &str) -> Self {
        let content_lower = content.to_lowercase();
        
        if content_lower.starts_with("http://") || content_lower.starts_with("https://") {
            // Проверка на платёжные URL
            if content_lower.contains("qr.nspk.ru") || content_lower.contains("pay") {
                ContentType::Payment
            } else {
                ContentType::Url
            }
        } else if content_lower.starts_with("begin:vcard") {
            ContentType::VCard
        } else if content_lower.starts_with("wifi:") {
            ContentType::WiFi
        } else if content_lower.starts_with("mailto:") {
            ContentType::Email
        } else if content_lower.starts_with("tel:") {
            ContentType::Phone
        } else if content_lower.starts_with("smsto:") || content_lower.starts_with("sms:") {
            ContentType::Sms
        } else if content_lower.starts_with("geo:") {
            ContentType::Geo
        } else if content.starts_with("00") && content.len() > 50 {
            // EMV QR обычно начинается с "00" (Payload Format Indicator)
            ContentType::Payment
        } else if content_lower.starts_with("st.") {
            // Российский стандарт ST.00012
            ContentType::Payment
        } else {
            ContentType::Text
        }
    }
}

/// Главный сканер QR-кодов
pub struct QRScanner {
    processor: ImageProcessor,
    detector: QRDetector,
    decoder: QRDecoder,
    payment_parser: PaymentParser,
}

impl Default for QRScanner {
    fn default() -> Self {
        Self::new()
    }
}

impl QRScanner {
    /// Создание нового сканера с настройками по умолчанию
    pub fn new() -> Self {
        Self {
            processor: ImageProcessor::new(ProcessingConfig::default()),
            detector: QRDetector::new(DetectorConfig::default()),
            decoder: QRDecoder::new(),
            payment_parser: PaymentParser::new(),
        }
    }
    
    /// Создание сканера с пользовательскими настройками
    pub fn with_config(
        processing: ProcessingConfig,
        detection: DetectorConfig,
    ) -> Self {
        Self {
            processor: ImageProcessor::new(processing),
            detector: QRDetector::new(detection),
            decoder: QRDecoder::new(),
            payment_parser: PaymentParser::new(),
        }
    }
    
    /// Сканирование изображения из байтов
    pub fn scan_bytes(&self, image_bytes: &[u8]) -> Result<ScanResult, QRError> {
        let start = std::time::Instant::now();
        
        // Загрузка изображения
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| QRError::InvalidFormat(e.to_string()))?;
        let gray = img.to_luma8();
        
        // Сканирование
        let result = self.scan_image(&gray, start);
        
        result
    }
    
    /// Сканирование GrayImage
    pub fn scan_image(&self, gray: &GrayImage, start: std::time::Instant) -> Result<ScanResult, QRError> {
        // Предобработка
        let processed = self.processor.process(gray);
        
        // Детекция QR-кодов
        let detected = self.detector.detect(&processed);
        
        // Декодирование каждого QR
        let mut qr_codes = Vec::new();
        let mut best_payment_score = 0.0f32;
        let mut best_payment_idx = None;
        
        for (idx, detection) in detected.iter().enumerate() {
            // Пробуем декодировать
            match self.decoder.decode(&detection.image) {
                Ok(decoded) => {
                    let content_type = ContentType::detect(&decoded.content);
                    let payment = if content_type == ContentType::Payment {
                        self.payment_parser.parse(&decoded.content)
                    } else {
                        None
                    };
                    
                    // Оценка релевантности для оплаты
                    let payment_score = self.payment_parser.relevance_score(&decoded.content);
                    if payment_score > best_payment_score {
                        best_payment_score = payment_score;
                        best_payment_idx = Some(idx);
                    }
                    
                    qr_codes.push(QRResult {
                        content: decoded.content,
                        bbox: detection.bbox,
                        content_type,
                        payment,
                        confidence: detection.confidence,
                    });
                }
                Err(e) => {
                    log::debug!("Failed to decode QR at {:?}: {}", detection.bbox, e);
                }
            }
        }
        
        // Если не нашли QR через детектор, пробуем декодировать всё изображение напрямую
        if qr_codes.is_empty() {
            if let Ok(decoded) = self.decoder.decode(&processed) {
                let content_type = ContentType::detect(&decoded.content);
                let payment = if content_type == ContentType::Payment {
                    self.payment_parser.parse(&decoded.content)
                } else {
                    None
                };
                
                qr_codes.push(QRResult {
                    content: decoded.content,
                    bbox: [0, 0, processed.width(), processed.height()],
                    content_type,
                    payment,
                    confidence: 1.0,
                });
                
                if best_payment_idx.is_none() && qr_codes.last().map(|q| q.content_type == ContentType::Payment).unwrap_or(false) {
                    best_payment_idx = Some(0);
                }
            }
        }
        
        Ok(ScanResult {
            qr_codes,
            best_payment: best_payment_idx,
            processing_time_ms: start.elapsed().as_millis() as u64,
        })
    }
    
    /// Сканирование с приоритетом платёжных QR
    pub fn scan_for_payment(&self, image_bytes: &[u8]) -> Result<Option<PaymentInfo>, QRError> {
        let result = self.scan_bytes(image_bytes)?;
        
        if let Some(idx) = result.best_payment {
            Ok(result.qr_codes.get(idx).and_then(|qr| qr.payment.clone()))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_content_type_detection() {
        assert_eq!(ContentType::detect("https://example.com"), ContentType::Url);
        assert_eq!(ContentType::detect("https://qr.nspk.ru/123"), ContentType::Payment);
        assert_eq!(ContentType::detect("BEGIN:VCARD\nVERSION:3.0"), ContentType::VCard);
        assert_eq!(ContentType::detect("WIFI:T:WPA;S:MyNetwork;P:pass;;"), ContentType::WiFi);
        assert_eq!(ContentType::detect("Hello World"), ContentType::Text);
    }
}
