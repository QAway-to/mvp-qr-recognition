//! WASM bindings для QR-сканера
//!
//! Предоставляет JavaScript API для распознавания QR-кодов

use qr_core::{QRScanner, ScanResult, ProcessingConfig, DetectorConfig};
use wasm_bindgen::prelude::*;
use serde_wasm_bindgen;

/// Инициализация panic hook для отладки
#[wasm_bindgen(start)]
pub fn init() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(log::Level::Debug).ok();
    log::info!("QR Scanner WASM module initialized");
}

/// JavaScript-доступный сканер QR-кодов
#[wasm_bindgen]
pub struct WasmQRScanner {
    scanner: QRScanner,
}

#[wasm_bindgen]
impl WasmQRScanner {
    /// Создание нового сканера с настройками по умолчанию
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            scanner: QRScanner::new(),
        }
    }
    
    /// Создание сканера с пользовательскими настройками
    #[wasm_bindgen(js_name = withConfig)]
    pub fn with_config(
        adaptive_threshold: bool,
        block_size: u32,
        denoise: bool,
        denoise_strength: f32,
        enhance_contrast: bool,
    ) -> Self {
        let processing = ProcessingConfig {
            adaptive_threshold,
            block_size,
            denoise,
            denoise_strength,
            enhance_contrast,
        };
        
        let detection = DetectorConfig::default();
        
        Self {
            scanner: QRScanner::with_config(processing, detection),
        }
    }
    
    /// Сканирование изображения из байтов (PNG, JPEG)
    /// 
    /// @param image_data - Uint8Array с данными изображения
    /// @returns Object с результатами сканирования
    #[wasm_bindgen(js_name = scanImage)]
    pub fn scan_image(&self, image_data: &[u8]) -> Result<JsValue, JsError> {
        match self.scanner.scan_bytes(image_data) {
            Ok(result) => {
                serde_wasm_bindgen::to_value(&result)
                    .map_err(|e| JsError::new(&e.to_string()))
            }
            Err(e) => Err(JsError::new(&e.to_string())),
        }
    }
    
    /// Сканирование ImageData из Canvas
    /// 
    /// @param data - Uint8ClampedArray из canvas.getImageData()
    /// @param width - Ширина изображения
    /// @param height - Высота изображения
    /// @returns Object с результатами сканирования
    #[wasm_bindgen(js_name = scanImageData)]
    pub fn scan_image_data(
        &self,
        data: &[u8],
        width: u32,
        height: u32,
    ) -> Result<JsValue, JsError> {
        // Конвертируем RGBA в Grayscale
        let gray = self.rgba_to_gray(data, width, height);
        
        // Создаём GrayImage
        let img = match image::GrayImage::from_raw(width, height, gray) {
            Some(img) => img,
            None => return Err(JsError::new("Failed to create image from data")),
        };
        
        // Сканируем
        // Сканируем
        match self.scanner.scan_image(&img) {
            Ok(result) => {
                serde_wasm_bindgen::to_value(&result)
                    .map_err(|e| JsError::new(&e.to_string()))
            }
            Err(e) => Err(JsError::new(&e.to_string())),
        }
    }
    
    /// Поиск платёжного QR-кода
    /// 
    /// @param image_data - Uint8Array с данными изображения
    /// @returns PaymentInfo или null
    #[wasm_bindgen(js_name = scanForPayment)]
    pub fn scan_for_payment(&self, image_data: &[u8]) -> Result<JsValue, JsError> {
        match self.scanner.scan_for_payment(image_data) {
            Ok(Some(payment)) => {
                serde_wasm_bindgen::to_value(&payment)
                    .map_err(|e| JsError::new(&e.to_string()))
            }
            Ok(None) => Ok(JsValue::NULL),
            Err(e) => Err(JsError::new(&e.to_string())),
        }
    }

    /// Загрузка ML модели (ONNX)
    /// 
    /// @param model_data - Uint8Array с байтами модели (.onnx)
    #[wasm_bindgen(js_name = loadModel)]
    pub fn load_model(&mut self, model_data: &[u8]) -> Result<(), JsError> {
        let detector = qr_core::OnnxDetector::load(model_data)
           .map_err(|e| JsError::new(&e.to_string()))?;
        
        self.scanner.set_ml_detector(detector);
        Ok(())
    }
    
    /// Конвертация RGBA в Grayscale
    fn rgba_to_gray(&self, rgba: &[u8], width: u32, height: u32) -> Vec<u8> {
        let pixel_count = (width * height) as usize;
        let mut gray = Vec::with_capacity(pixel_count);
        
        for i in 0..pixel_count {
            let offset = i * 4;
            if offset + 2 < rgba.len() {
                let r = rgba[offset] as f32;
                let g = rgba[offset + 1] as f32;
                let b = rgba[offset + 2] as f32;
                // ITU-R BT.601 luma formula
                let luma = (0.299 * r + 0.587 * g + 0.114 * b) as u8;
                gray.push(luma);
            }
        }
        
        gray
    }
}

impl Default for WasmQRScanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Удобная функция для быстрого сканирования
#[wasm_bindgen(js_name = quickScan)]
pub fn quick_scan(image_data: &[u8]) -> Result<JsValue, JsError> {
    let scanner = WasmQRScanner::new();
    scanner.scan_image(image_data)
}

/// Информация о версии
#[wasm_bindgen(js_name = version)]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_bindgen_test::*;
    
    wasm_bindgen_test_configure!(run_in_browser);
    
    #[wasm_bindgen_test]
    fn test_scanner_creation() {
        let _scanner = WasmQRScanner::new();
    }
    
    #[wasm_bindgen_test]
    fn test_version() {
        let v = version();
        assert!(!v.is_empty());
    }
}
