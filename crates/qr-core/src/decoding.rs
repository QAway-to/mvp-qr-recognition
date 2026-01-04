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
        if let Ok(result) = self.decode_with_rqrr(img) {
            return Ok(result);
        }

        // Пробуем rxing (может паниковать из-за таймеров в WASM, поэтому второй)
        // ENABLED for WASM with default-features=false
        if let Ok(result) = self.decode_with_rxing(img) {
            return Ok(result);
        }
        
        // Пробуем инвертированное изображение
        if self.try_inverted {
            let inverted = self.invert_image(img);
            
            // ENABLED for WASM with default-features=false
            if let Ok(result) = self.decode_with_rxing(&inverted) {
                return Ok(result);
            }
            
            if let Ok(result) = self.decode_with_rqrr(&inverted) {
                return Ok(result);
            }
        }
        
        Err(DecodeError::NotFound)
    }
    
    /// Декодирование через rxing
    fn decode_with_rxing(&self, img: &GrayImage) -> Result<DecodedQR, DecodeError> {
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
        // TryHarder может использовать таймеры, что вызывает панику в WASM без поддержки времени
        // hints.insert(
        //     rxing::DecodeHintType::TRY_HARDER,
        //     rxing::DecodeHintValue::TryHarder(true),
        // );
        
        let mut reader = QRCodeReader::new();
        
        match reader.decode_with_hints(&mut bitmap, &hints) {
            Ok(result) => {
                Ok(DecodedQR {
                    content: result.getText().to_string(),
                    error_correction: ErrorCorrectionLevel::Unknown,
                    version: None,
                    encoding: format!("{:?}", result.getBarcodeFormat()),
                })
            }
            Err(Exceptions::NotFoundException(_)) => Err(DecodeError::NotFound),
            Err(e) => Err(DecodeError::DecodeFailed(e.to_string())),
        }
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
