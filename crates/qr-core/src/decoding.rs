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
        // 1. Стандартное декодирование
        // Пробуем rqrr сначала (более стабилен для WASM)
        if let Ok(result) = self.decode_with_rqrr(img) {
            return Ok(result);
        }

        // Пробуем rxing. В V14 мы убираем ограничение strong_hint для GlobalHistogram,
        // чтобы вернуть максимальную надежность. Всегда пробуем все методы.
        if let Ok(result) = self.decode_with_rxing(img, true) {
            return Ok(result);
        }
        
        // 2. Инвертированное изображение
        if self.try_inverted {
            log::info!("FALLBACK: Trying inverted image...");
            let inverted = self.invert_image(img);
            
            if let Ok(result) = self.decode_with_rqrr(&inverted) {
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&inverted, true) {
                return Ok(result);
            }
        }

        // 3. Улучшенное изображение (Контраст + Резкость)
        log::info!("FALLBACK: Standard/Inverted failed. Trying Advanced Preprocessing (Contrast + Sharpen)...");
        let preprocessed = self.preprocess_image(img);

        // a) Standard Preprocessed
        if let Ok(result) = self.decode_with_rqrr(&preprocessed) {
            log::info!("SUCCESS: Advanced Preprocessing + RQRR worked!");
            return Ok(result);
        }
        if let Ok(result) = self.decode_with_rxing(&preprocessed, true) {
            log::info!("SUCCESS: Advanced Preprocessing + RXING worked!");
            return Ok(result);
        }

        // b) Inverted Preprocessed
        if self.try_inverted {
            log::info!("FALLBACK: Trying Preprocessed + Inverted...");
            let prep_inverted = self.invert_image(&preprocessed);
            
            if let Ok(result) = self.decode_with_rqrr(&prep_inverted) {
                log::info!("SUCCESS: Preprocessed+Inverted + RQRR worked!");
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&prep_inverted, true) {
                log::info!("SUCCESS: Preprocessed+Inverted + RXING worked!");
                return Ok(result);
            }
        }

        // 4. Add Padding Fallback (V17 - Quiet Zone Restoration)
        // Если изображение обрезано слишком близко к QR-коду (особенно при повороте),
        // добавляем белую рамку (Quiet Zone).
        log::info!("FALLBACK: Trying Padding (Quiet Zone Restoration)...");
        let padded = self.add_white_padding(img, 20); // 20px padding
        if let Ok(result) = self.decode_with_rqrr(&padded) {
            log::info!("SUCCESS: Padding + RQRR worked!");
            return Ok(result);
        }
        if let Ok(result) = self.decode_with_rxing(&padded, true) {
            log::info!("SUCCESS: Padding + RXING worked!");
            return Ok(result);
        }

        // Также пробуем инвертированный вариант с padding (на случай черного фона)
        if self.try_inverted {
            log::info!("FALLBACK: Trying Padding + Inverted...");
            // Инвертируем СНАЧАЛА, потом добавляем паддинг (чтобы был белый фон вокруг инвертированного QR)
            // Но если QR был "белый на черном", то после инверсии он стал "черный на белом".
            // Значит паддинг должен быть БЕЛЫМ.
            let inverted = self.invert_image(img);
            let padded_inverted = self.add_white_padding(&inverted, 20);
            
            if let Ok(result) = self.decode_with_rqrr(&padded_inverted) {
                log::info!("SUCCESS: Padding + Inverted + RQRR worked!");
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&padded_inverted, true) {
                log::info!("SUCCESS: Padding + Inverted + RXING worked!");
                return Ok(result);
            }
        }



        // 5. Rotation Fallback (V18)
        // Если изображение повернуто под экзотическим углом (например 45 градусов),
        // стандартные сканеры могут не справиться. Мы поворачиваем изображение, чтобы выровнять QR.
        // Пробуем 45, 30, 60 градусов.
        // 5. Rotation Fallback (V18)
        // Если изображение повернуто под экзотическим углом (например 45 градусов),
        // стандартные сканеры могут не справиться. Мы поворачиваем изображение, чтобы выровнять QR.
        // Пробуем 45, 30, 60 градусов (и отрицательные).
        log::info!("FALLBACK: Trying Rotation (30, 45, 60)...");
        let angles = [30.0, 45.0, 60.0];
        
        for angle in angles {
            // Try +angle
            let rotated = self.rotate_image(img, angle);
            
            // Try Standard on rotated
            if let Ok(result) = self.decode_with_rqrr(&rotated) {
                log::info!("SUCCESS: Rotation ({} deg) + RQRR worked!", angle);
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&rotated, true) {
                log::info!("SUCCESS: Rotation ({} deg) + RXING worked!", angle);
                return Ok(result);
            }
        }

        // 6. Multi-Threshold Fallback (V16)
        // Пробуем несколько порогов бинаризации, включая автоматический (Otsu).
        let otsu_threshold = self.calculate_otsu_threshold(img);
        log::info!("FALLBACK: Trying Multi-Threshold (Otsu={}, 64, 96, 128, 160, 192)...", otsu_threshold);
        
        let thresholds: [u8; 6] = [otsu_threshold, 64, 96, 128, 160, 192];
        
        for threshold in thresholds {
            let thresholded = self.apply_threshold(img, threshold);
            if let Ok(result) = self.decode_with_rqrr(&thresholded) {
                log::info!("SUCCESS: Multi-Threshold ({}) + RQRR worked!", threshold);
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&thresholded, true) {
                log::info!("SUCCESS: Multi-Threshold ({}) + RXING worked!", threshold);
                return Ok(result);
            }
        }

        // 7. Downscale Fallback (V16)
        if img.width() > 400 || img.height() > 400 {
            log::info!("FALLBACK: Trying Downscale (50%)...");
            let downscaled = self.downscale_image(img, 2);
            if let Ok(result) = self.decode_with_rqrr(&downscaled) {
                log::info!("SUCCESS: Downscale + RQRR worked!");
                return Ok(result);
            }
            if let Ok(result) = self.decode_with_rxing(&downscaled, true) {
                log::info!("SUCCESS: Downscale + RXING worked!");
                return Ok(result);
            }
        }
        
        Err(DecodeError::NotFound)
    }

    /// Добавляет белую рамку вокруг изображения
    fn add_white_padding(&self, img: &GrayImage, padding: u32) -> GrayImage {
        let (width, height) = img.dimensions();
        let new_width = width + padding * 2;
        let new_height = height + padding * 2;
        
        // Создаем изображение, заполненное белым (255)
        let mut padded = GrayImage::from_pixel(new_width, new_height, image::Luma([255]));
        
        // Копируем исходное изображение в центр
        image::imageops::overlay(&mut padded, img, padding as i64, padding as i64);
        
        padded
    }


    // ... (skipping to function definition)

    /// Поворачивает изображение на заданный угол (в градусах) с изменением размера холста,
    /// чтобы углы не обрезались. Заполняет фон белым (Quiet Zone).
    fn rotate_image(&self, img: &GrayImage, angle_degrees: f32) -> GrayImage {
        let (w, h) = img.dimensions();
        let rad = angle_degrees.to_radians();
        let cos_a = rad.cos();
        let sin_a = rad.sin();
        
        // Calculate new dimensions to fit the rotated image (Bounding Box)
        let new_w = (w as f32 * cos_a.abs() + h as f32 * sin_a.abs()).ceil() as u32;
        let new_h = (w as f32 * sin_a.abs() + h as f32 * cos_a.abs()).ceil() as u32;
        
        // Centers
        let cx = w as f32 / 2.0;
        let cy = h as f32 / 2.0;
        let new_cx = new_w as f32 / 2.0;
        let new_cy = new_h as f32 / 2.0;
        
        // Create new white image (Quiet Zone)
        let mut new_img = GrayImage::from_pixel(new_w, new_h, image::Luma([255]));

        for y in 0..new_h {
            for x in 0..new_w {
                // Shift to center
                let dx = x as f32 - new_cx;
                let dy = y as f32 - new_cy;
                
                // Rotate back (inverse transform)
                // x_src = dx * cos + dy * sin
                // y_src = -dx * sin + dy * cos
                let src_x = dx * cos_a + dy * sin_a + cx;
                let src_y = -dx * sin_a + dy * cos_a + cy;
                
                // Check bounds using Nearest Neighbor
                if src_x >= 0.0 && src_x < (w as f32 - 0.5) && src_y >= 0.0 && src_y < (h as f32 - 0.5) {
                    let sx = src_x.round() as u32;
                    let sy = src_y.round() as u32;
                    if sx < w && sy < h {
                         new_img.put_pixel(x, y, *img.get_pixel(sx, sy));
                    }
                }
            }
        }
        new_img
    }

    /// Предобработка: Растяжение контраста + Повышение резкости
    fn preprocess_image(&self, img: &GrayImage) -> GrayImage {
        // 1. Растяжение контраста (нормализация гистограммы)
        let contrast_img = self.apply_contrast_stretch(img);

        // 2. Повышение резкости (Sharpening)
        // Используем стандартный 3x3 фильтр для выделения краев модулей QR кода
        self.apply_sharpen(&contrast_img)
    }

    fn apply_contrast_stretch(&self, img: &GrayImage) -> GrayImage {
        let mut min_val = 255u8;
        let mut max_val = 0u8;

        for p in img.pixels() {
            let val = p.0[0];
            if val < min_val { min_val = val; }
            if val > max_val { max_val = val; }
        }

        if min_val >= max_val {
            return img.clone();
        }

        let (width, height) = img.dimensions();
        let mut result = GrayImage::new(width, height);
        
        let range = (max_val - min_val) as f32;

        for (x, y, p) in img.enumerate_pixels() {
            let val = p.0[0];
            // (val - min) / (max - min) * 255
            let new_val = ((val as f32 - min_val as f32) / range * 255.0) as u8;
            result.put_pixel(x, y, image::Luma([new_val]));
        }
        result
    }

    fn apply_sharpen(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        // Start with a copy to preserve borders
        let mut result = img.clone(); 
        
        // Kernel:
        //  0 -1  0
        // -1  5 -1
        //  0 -1  0
        
        for y in 1..height-1 {
            for x in 1..width-1 {
                // Unsafe get is slightly faster but safe get is fine here
                let val = (img.get_pixel(x, y).0[0] as i32 * 5)
                        - (img.get_pixel(x, y-1).0[0] as i32)
                        - (img.get_pixel(x, y+1).0[0] as i32)
                        - (img.get_pixel(x-1, y).0[0] as i32)
                        - (img.get_pixel(x+1, y).0[0] as i32);
                
                let clamped = val.max(0).min(255) as u8;
                result.put_pixel(x, y, image::Luma([clamped]));
            }
        }
        result
    }

    /// Жесткая бинаризация по порогу
    fn apply_threshold(&self, img: &GrayImage, threshold: u8) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = GrayImage::new(width, height);
        
        for (x, y, p) in img.enumerate_pixels() {
            let val = if p.0[0] < threshold { 0 } else { 255 };
            result.put_pixel(x, y, image::Luma([val]));
        }
        result
    }

    /// Вычисление порога по методу Otsu (минимизация внутриклассовой дисперсии)
    fn calculate_otsu_threshold(&self, img: &GrayImage) -> u8 {
        // Строим гистограмму
        let mut histogram = [0u32; 256];
        let total_pixels = (img.width() * img.height()) as f64;
        
        for p in img.pixels() {
            histogram[p.0[0] as usize] += 1;
        }

        let mut sum: f64 = 0.0;
        for (i, &count) in histogram.iter().enumerate() {
            sum += i as f64 * count as f64;
        }

        let mut sum_b: f64 = 0.0;
        let mut w_b: f64 = 0.0;
        let mut max_variance: f64 = 0.0;
        let mut threshold: u8 = 128; // Default fallback

        for (t, &count) in histogram.iter().enumerate() {
            w_b += count as f64;
            if w_b == 0.0 { continue; }
            
            let w_f = total_pixels - w_b;
            if w_f == 0.0 { break; }

            sum_b += t as f64 * count as f64;
            
            let m_b = sum_b / w_b;
            let m_f = (sum - sum_b) / w_f;
            
            let variance = w_b * w_f * (m_b - m_f) * (m_b - m_f);
            
            if variance > max_variance {
                max_variance = variance;
                threshold = t as u8;
            }
        }
        
        threshold
    }

    /// Уменьшение изображения в заданное число раз (простое усреднение)
    fn downscale_image(&self, img: &GrayImage, factor: u32) -> GrayImage {
        let new_width = img.width() / factor;
        let new_height = img.height() / factor;
        let mut result = GrayImage::new(new_width, new_height);

        for y in 0..new_height {
            for x in 0..new_width {
                let mut sum: u32 = 0;
                for dy in 0..factor {
                    for dx in 0..factor {
                        sum += img.get_pixel(x * factor + dx, y * factor + dy).0[0] as u32;
                    }
                }
                let avg = (sum / (factor * factor)) as u8;
                result.put_pixel(x, y, image::Luma([avg]));
            }
        }
        result
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
