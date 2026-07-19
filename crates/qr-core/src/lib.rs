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
#[cfg(feature = "ml")]
pub mod ml_detection;
pub mod emv;
pub mod emv_parser;
pub mod geometry;

pub use preprocessing::{ImageProcessor, ProcessingConfig};
pub use detection::{QRDetector, DetectedQR, DetectorConfig};
pub use decoding::{QRDecoder, DecodedQR, DecodeError};
pub use payment::{PaymentParser, PaymentInfo, PaymentFormat};
#[cfg(feature = "ml")]
pub use ml_detection::OnnxDetector;
// pub use emv::EmvData; // Deprecated?
pub use emv_parser::{EmvData as EmvDataV19, parse_emv_qr};

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

    /// Установка ML детектора
    #[cfg(feature = "ml")]
    pub fn set_ml_detector(&mut self, detector: OnnxDetector) {
        self.detector.set_ml_detector(detector);
    }
    
    /// Сканирование изображения из байтов
    /// Сканирование изображения из байтов
    pub fn scan_bytes(&self, image_bytes: &[u8]) -> Result<ScanResult, QRError> {
        // Загрузка изображения
        let img = image::load_from_memory(image_bytes)
            .map_err(|e| QRError::InvalidFormat(e.to_string()))?;
        let gray = img.to_luma8();
        
        // Сканирование
        self.scan_image(&gray)
    }
    
    /// Сканирование GrayImage
    pub fn scan_image(&self, gray: &GrayImage) -> Result<ScanResult, QRError> {
        log::info!("Starting scan_image, size: {:?}", gray.dimensions());

        // Предобработка
        log::info!("Starting preprocessing");
        let processed = self.processor.process(gray);
        log::info!("Preprocessing done, resulting size: {:?}", processed.dimensions());
        
        let mut qr_codes = Vec::new();
        let mut best_payment_score = 0.0f32;
        let mut best_payment_idx = None;

        // --- Helper for decoding detections ---
        // Warning: This updates qr_codes, best_payment_score, best_payment_idx.
        // We can't easily make it a closure due to ownership, so we'll use a loop.
        
        // 1. Initial Detection (0 degrees)
        log::info!("Starting detection (0 deg)");
        let detected_0 = self.detector.detect(&processed, None); // Standard threshold
        log::info!("Detection (0 deg) done, found: {}", detected_0.len());
        
        // Decode 0 deg
        for (idx, detection) in detected_0.iter().enumerate() {
            log::info!("Decoding detected QR #{}", idx);
            match self.decoder.decode(&detection.image) {
                Ok(decoded) => {
                    log::info!("Decoded successfully: {:?}", decoded.content);
                    let content_type = ContentType::detect(&decoded.content);
                    let payment = if content_type == ContentType::Payment {
                        self.payment_parser.parse(&decoded.content)
                    } else {
                        None
                    };
                    
                    let payment_score = self.payment_parser.relevance_score(&decoded.content);
                    if payment_score > best_payment_score {
                        best_payment_score = payment_score;
                        best_payment_idx = Some(qr_codes.len()); // Use current len
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

        // 2. Rotation Fallback (V19)
        // If we found nothing so far, try rotating 45 degrees.
        // This handles cases where ML fails to detect rotated QRs OR ML detects them but decoder fails on rotated crop.
        if qr_codes.is_empty() {
             log::info!("No codes found yet. Trying 45 degree rotation fallback...");
             let rotated = geometry::rotate_image(&processed, 45.0);
             let rotated_detected = self.detector.detect(&rotated, None);
             log::info!("Detection (45 deg) done, found: {}", rotated_detected.len());
             
             let (w, h) = processed.dimensions();

             for (idx, detection) in rotated_detected.iter().enumerate() {
                // Determine Original BBox (approximate)
                // We map the crop back.
                // Note: The 'detection.image' is UPRIGHT (because we rotated the image 45 deg).
                // So decoding it should trigger standard decoder success.
                
                match self.decoder.decode(&detection.image) {
                    Ok(decoded) => {
                        log::info!("Rotation Fallback: Decoded successfully: {:?}", decoded.content);
                        
                        // Map bbox/corners back to 0 deg frame
                        let mut new_corners = [(0,0); 4];
                        for (i, c) in detection.corners.iter().enumerate() {
                            let (mx, my) = geometry::map_rotated_point_back(
                                c.0 as f32, c.1 as f32, w, h, 45.0
                            );
                            new_corners[i] = (mx as u32, my as u32);
                        }
                        
                        let xs: Vec<u32> = new_corners.iter().map(|c| c.0).collect();
                        let ys: Vec<u32> = new_corners.iter().map(|c| c.1).collect();
                        let min_x = *xs.iter().min().unwrap_or(&0);
                        let max_x = *xs.iter().max().unwrap_or(&0);
                        let min_y = *ys.iter().min().unwrap_or(&0);
                        let max_y = *ys.iter().max().unwrap_or(&0);
                        
                        let mapped_bbox = [min_x, min_y, max_x - min_x, max_y - min_y];
                        
                        let content_type = ContentType::detect(&decoded.content);
                        let payment = if content_type == ContentType::Payment {
                            self.payment_parser.parse(&decoded.content)
                        } else {
                            None
                        };
                        
                        let payment_score = self.payment_parser.relevance_score(&decoded.content);
                        if payment_score > best_payment_score {
                            best_payment_score = payment_score;
                            best_payment_idx = Some(qr_codes.len());
                        }
                        
                        qr_codes.push(QRResult {
                            content: decoded.content,
                            bbox: mapped_bbox,
                            content_type,
                            payment,
                            confidence: detection.confidence,
                        });
                    }
                    Err(e) => {
                         log::debug!("Rotation Fallback: Failed to decode QR #{}: {}", idx, e);
                    }
                }
             }
        }
        
        // 3. Last Resort: Full Image Decode (if still empty)
        if qr_codes.is_empty() {
            log::info!("No QRs found via detection (0 or 45), trying full image decode");
            if let Ok(decoded) = self.decoder.decode(&processed) {
                log::info!("Full image decode success: {:?}", decoded.content);
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
            } else {
                log::info!("Full image decode failed");
            }
        }
        
        log::info!("Scan complete, found {} codes", qr_codes.len());
        
        Ok(ScanResult {
            qr_codes,
            best_payment: best_payment_idx,
            processing_time_ms: 0,
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
