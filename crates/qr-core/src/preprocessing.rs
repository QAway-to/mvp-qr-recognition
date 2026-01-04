//! Модуль предобработки изображений
//! 
//! Функции для улучшения качества изображения перед распознаванием QR:
//! - Адаптивная бинаризация (отключено в V14)
//! - Подавление шумов (отключено в V14)
//! - Повышение контрастности (отключено в V14)
//! - Нормализация освещения (отключено в V14)

use image::{GrayImage, Luma};
use serde::{Deserialize, Serialize};
use nalgebra; // Required for find_corners signature

/// Конфигурация предобработки
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessingConfig {
    /// Включить адаптивную бинаризацию
    pub adaptive_threshold: bool,
    /// Размер блока для адаптивной бинаризации (нечётное число)
    pub block_size: u32,
    /// Включить шумоподавление
    pub denoise: bool,
    /// Сила шумоподавления (sigma для Гаусса)
    pub denoise_strength: f32,
    /// Включить повышение контрастности
    pub enhance_contrast: bool,
}

impl Default for ProcessingConfig {
    fn default() -> Self {
        Self {
            adaptive_threshold: true,
            block_size: 51,
            denoise: true,
            denoise_strength: 1.0,
            enhance_contrast: true,
        }
    }
}

/// Процессор изображений
pub struct ImageProcessor {
    config: ProcessingConfig,
}

impl ImageProcessor {
    /// Создание процессора с конфигурацией
    pub fn new(config: ProcessingConfig) -> Self {
        Self { config }
    }
    
    /// Полная обработка изображения
    pub fn process(&self, img: &GrayImage) -> GrayImage {
        // 0. Resize if too large (improves performance and consistency)
        let mut result = self.resize(img, 1000); // Max 1000px
        
        // В V14 мы полагаемся на встроенный fallback в decoding.rs
        // Поэтому здесь просто возвращаем ресайзнутое изображение
        // Методы оставлены для совместимости API.
        
        result
    }
    
    /// Адаптивная бинаризация (Stub)
    pub fn adaptive_threshold(&self, img: &GrayImage) -> GrayImage {
        img.clone()
    }
    
    /// Подавление шумов (Stub)
    pub fn denoise(&self, img: &GrayImage) -> GrayImage {
        img.clone()
    }
    
    /// Медианный фильтр для удаления импульсного шума (Stub)
    pub fn median_denoise(&self, img: &GrayImage) -> GrayImage {
        img.clone()
    }
    
    /// Повышение контрастности (Stub)
    pub fn enhance_contrast(&self, img: &GrayImage) -> GrayImage {
        img.clone()
    }
    
    /// Нормализация освещения через локальное выравнивание (Stub)
    pub fn normalize_lighting(&self, img: &GrayImage) -> GrayImage {
        img.clone()
    }
    
    /// Инвертирование изображения (для QR с инвертированными цветами)
    pub fn invert(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut result = GrayImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let pixel = img.get_pixel(x, y).0[0];
                result.put_pixel(x, y, Luma([255 - pixel]));
            }
        }
        
        result
    }
    
    /// Ресайз изображения с сохранением пропорций
    pub fn resize(&self, img: &GrayImage, max_dimension: u32) -> GrayImage {
        let (width, height) = img.dimensions();
        
        if width <= max_dimension && height <= max_dimension {
            return img.clone();
        }
        
        let scale = if width > height {
            max_dimension as f32 / width as f32
        } else {
            max_dimension as f32 / height as f32
        };
        
        let new_width = (width as f32 * scale) as u32;
        let new_height = (height as f32 * scale) as u32;
        
        image::imageops::resize(
            img,
            new_width,
            new_height,
            image::imageops::FilterType::Triangle,
        )
    }

    /// Find corners of the QR code within the image (or ROI)
    /// Returns 4 points [TL, TR, BR, BL] if a valid quad is found.
    pub fn find_corners(&self, _img: &GrayImage) -> Option<[nalgebra::Point2<f32>; 4]> {
        // Disabled in V14 due to dependency removal
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_processor_creation() {
        let config = ProcessingConfig::default();
        let _processor = ImageProcessor::new(config);
    }
    
    #[test]
    fn test_invert() {
        let processor = ImageProcessor::new(ProcessingConfig::default());
        let img = GrayImage::from_pixel(10, 10, Luma([100]));
        let inverted = processor.invert(&img);
        assert_eq!(inverted.get_pixel(0, 0).0[0], 155);
    }
}
