//! Модуль декодирования QR-кодов
//!
//! Использует rxing как основной декодер с fallback на rqrr

use image::GrayImage;
use rxing::{BarcodeFormat, DecodingHintDictionary, Exceptions, Reader};
use rxing::qrcode::QRCodeReader;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Ошибки декодирования
#[derive(Error, Debug)]
pub enum DecodeError {
    #[error("No QR code found in image")]
    NotFound,
    
    #[error("Failed to decode QR: {0}")]
    DecodeFailed(String),
    
    #[error("Invalid image: {0}")]
    InvalidImage(String),
    
    #[error("Checksum error")]
    ChecksumError,
}

/// Уровень коррекции ошибок
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum ErrorCorrectionLevel {
    L, // ~7%
    M, // ~15%
    Q, // ~25%
    H, // ~30%
    Unknown,
}

/// Декодированный QR-код
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecodedQR {
    /// Декодированный текст
    pub content: String,
    /// Уровень коррекции ошибок
    pub error_correction: ErrorCorrectionLevel,
    /// Версия QR-кода (1-40)
    pub version: Option<u8>,
    /// Формат данных (Numeric, Alphanumeric, Byte, Kanji)
    pub encoding: String,
}

/// Декодер QR-кодов с fallback
pub struct QRDecoder {
    /// Попробовать инвертированное изображение
    try_inverted: bool,
}

impl Default for QRDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl QRDecoder {
    /// Создание декодера
    pub fn new() -> Self {
        Self {
            try_inverted: true,
        }
    }
    
    /// Декодирование QR-кода
    pub fn decode(&self, img: &GrayImage) -> Result<DecodedQR, DecodeError> {
        // Пробуем rqrr сначала (более стабилен для WASM)
        let rqrr_result = self.decode_with_rqrr(img);
        
        // Если rqrr нашел сетку, но не смог декодировать (DataEcc/FormatEcc), 
        // это сильный сигнал попробовать альтернативные методы бинаризации в rxing
        let strong_hint = matches!(rqrr_result, Err(DecodeError::DecodeFailed(_)));
        
        if let Ok(result) = rqrr_result {
            return Ok(result);
        }

        // Пробуем rxing
        // Включаем GlobalHistogramBinarizer только если rqrr что-то нашел, чтобы экономить CPU
        if let Ok(result) = self.decode_with_rxing(img, strong_hint) {
            return Ok(result);
        }
        
        // Пробуем инвертированное изображение
        if self.try_inverted {
            let inverted = self.invert_image(img);
            
            // Сначала rqrr для инвертированного (быстрее)
            let rqrr_inv_result = self.decode_with_rqrr(&inverted);
            let strong_hint_inv = matches!(rqrr_inv_result, Err(DecodeError::DecodeFailed(_)));
            
            if let Ok(result) = rqrr_inv_result {
                return Ok(result);
            }
            
            // Затем rxing с умным фоллбеком
            if let Ok(result) = self.decode_with_rxing(&inverted, strong_hint_inv) {
                return Ok(result);
            }
        }
        
        Err(DecodeError::NotFound)
    }
    
    /// Декодирование через rxing
    fn decode_with_rxing(&self, img: &GrayImage, try_fallback: bool) -> Result<DecodedQR, DecodeError> {
        log::info!("RXING: Starting decode on {}x{} image", img.width(), img.height());
        let (width, height) = img.dimensions();
        
        // Конвертируем grayscale в packed ARGB u32 формат для rxing
        // Формат: 0xAARRGGBB
        let pixels: Vec<u32> = img.as_raw()
            .iter()
            .map(|&gray| {
                let g = gray as u32;
                0xFF000000 | (g << 16) | (g << 8) | g  // ARGB with gray repeated
            })
            .collect();
        
        let luminance_source = rxing::RGBLuminanceSource::new_with_width_height_pixels(
            width as usize,
            height as usize,
            &pixels,
        );
        
        let mut bitmap = rxing::BinaryBitmap::new(rxing::common::HybridBinarizer::new(luminance_source));
        
        let mut hints = DecodingHintDictionary::new();
        hints.insert(
            rxing::DecodeHintType::POSSIBLE_FORMATS,
            rxing::DecodeHintValue::PossibleFormats(std::collections::HashSet::from([
                BarcodeFormat::QR_CODE,
            ])),
        );
        // TryHarder is now safe with chrono + wasmbind
        hints.insert(
            rxing::DecodeHintType::TRY_HARDER,
            rxing::DecodeHintValue::TryHarder(true),
        );
        
        let mut reader = QRCodeReader::new();
        
        // Попытка 1: HybridBinarizer (стандарт)
        match reader.decode_with_hints(&mut bitmap, &hints) {
            Ok(result) => {
                log::info!("RXING: Decode success (HybridBinarizer)!");
                return Ok(DecodedQR {
                    content: result.getText().to_string(),
                    error_correction: ErrorCorrectionLevel::Unknown,
                    version: None,
                    encoding: format!("{:?}", result.getBarcodeFormat()),
                });
            }
            Err(_) => {
                // Ignore error
            }
        }

        // Попытка 2: GlobalHistogramBinarizer (только если есть сильный сигнал)
        if try_fallback {
            log::info!("RXING: HybridBinarizer failed, trying GlobalHistogramBinarizer (strong hint)");
            
            let luminance_source_global = rxing::RGBLuminanceSource::new_with_width_height_pixels(
                width as usize,
                height as usize,
                &pixels,
            );
            let mut bitmap_global = rxing::BinaryBitmap::new(rxing::common::GlobalHistogramBinarizer::new(luminance_source_global));
    
            match reader.decode_with_hints(&mut bitmap_global, &hints) {
                Ok(result) => {
                    log::info!("RXING: Decode success (GlobalHistogramBinarizer)!");
                    return Ok(DecodedQR {
                        content: result.getText().to_string(),
                        error_correction: ErrorCorrectionLevel::Unknown,
                        version: None,
                        encoding: format!("{:?}", result.getBarcodeFormat()),
                    });
                }
                Err(e) => {
                    log::info!("RXING: GlobalHistogram failed: {}", e);
                }
            }
        } else {
             log::info!("RXING: Not found (HybridBinarizer)");
             return Err(DecodeError::NotFound);
        }
        
        log::info!("RXING: Not found");
        Err(DecodeError::NotFound)
    }
    
    /// Декодирование через rqrr (fallback)
    fn decode_with_rqrr(&self, img: &GrayImage) -> Result<DecodedQR, DecodeError> {
        log::info!("RQRR: Starting detection on {}x{} image", img.width(), img.height());
        let mut prepared = rqrr::PreparedImage::prepare(img.clone());
        let grids = prepared.detect_grids();
        log::info!("RQRR: Detected {} grids", grids.len());
        
        if grids.is_empty() {
            log::info!("RQRR: No grids found");
            return Err(DecodeError::NotFound);
        }
        
        // Берём первый найденный QR
        let grid = &grids[0];
        
        match grid.decode() {
            Ok((meta, content)) => {
                log::info!("RQRR: Decode success!");
                let error_correction = match meta.ecc_level {
                    0 => ErrorCorrectionLevel::L,
                    1 => ErrorCorrectionLevel::M,
                    2 => ErrorCorrectionLevel::Q,
                    3 => ErrorCorrectionLevel::H,
                    _ => ErrorCorrectionLevel::Unknown,
                };
                
                Ok(DecodedQR {
                    content,
                    error_correction,
                    version: Some(meta.version.0 as u8),
                    encoding: "Byte".to_string(),
                })
            }
            Err(e) => {
                log::info!("RQRR: Decode failed: {:?}", e);
                Err(DecodeError::DecodeFailed(format!("{:?}", e)))
            },
        }
    }
    
    /// Пакетное декодирование
    pub fn decode_batch(&self, images: &[GrayImage]) -> Vec<Result<DecodedQR, DecodeError>> {
        images.iter().map(|img| self.decode(img)).collect()
    }
    
    /// Инвертирование изображения
    fn invert_image(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = GrayImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y).0[0];
                result.put_pixel(x, y, image::Luma([255 - pixel]));
            }
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_decoder_creation() {
        let _decoder = QRDecoder::new();
    }
}
