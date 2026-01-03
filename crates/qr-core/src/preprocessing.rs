//! Модуль предобработки изображений
//! 
//! Функции для улучшения качества изображения перед распознаванием QR:
//! - Адаптивная бинаризация
//! - Подавление шумов
//! - Повышение контрастности
//! - Нормализация освещения

use image::{GrayImage, Luma};
use imageproc::contrast::{adaptive_threshold, stretch_contrast};
use imageproc::filter::{gaussian_blur_f32, median_filter};
use serde::{Deserialize, Serialize};

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
        let mut result = img.clone();
        
        // 1. Шумоподавление (если включено)
        if self.config.denoise {
            result = self.denoise(&result);
        }
        
        // 2. Повышение контрастности (если включено)
        if self.config.enhance_contrast {
            result = self.enhance_contrast(&result);
        }
        
        // 3. Адаптивная бинаризация (если включена)
        if self.config.adaptive_threshold {
            result = self.adaptive_threshold(&result);
        }
        
        result
    }
    
    /// Адаптивная бинаризация (Bradley/Otsu)
    pub fn adaptive_threshold(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        
        // Safety check: block_size must be less than image dimensions
        let max_block = width.min(height).saturating_sub(1);
        if max_block < 3 {
            // Image too small for adaptive threshold, return as-is
            return img.clone();
        }
        
        // block_size должен быть нечётным и меньше размера изображения
        let mut block_size = self.config.block_size.min(max_block);
        if block_size % 2 == 0 {
            block_size = block_size.saturating_sub(1).max(3);
        }
        if block_size < 3 {
            block_size = 3;
        }
        
        adaptive_threshold(img, block_size)
    }
    
    /// Подавление шумов (Гауссово размытие + медианный фильтр)
    pub fn denoise(&self, img: &GrayImage) -> GrayImage {
        // Лёгкое Гауссово размытие для подавления высокочастотного шума
        let sigma = self.config.denoise_strength;
        
        if sigma > 0.0 {
            gaussian_blur_f32(img, sigma)
        } else {
            img.clone()
        }
    }
    
    /// Медианный фильтр для удаления импульсного шума
    pub fn median_denoise(&self, img: &GrayImage) -> GrayImage {
        // Радиус 1 = окно 3x3
        let radius_x = 1;
        let radius_y = 1;
        median_filter(img, radius_x, radius_y)
    }
    
    /// Повышение контрастности (растяжка гистограммы)
    pub fn enhance_contrast(&self, img: &GrayImage) -> GrayImage {
        // Растяжка контраста на весь динамический диапазон
        stretch_contrast(img, 0, 255)
    }
    
    /// Нормализация освещения через локальное выравнивание
    pub fn normalize_lighting(&self, img: &GrayImage) -> GrayImage {
        // Вычисляем локальное среднее с большим размером окна
        let blurred = gaussian_blur_f32(img, 20.0);
        
        let (width, height) = img.dimensions();
        let mut result = GrayImage::new(width, height);
        
        for y in 0..height {
            for x in 0..width {
                let original = img.get_pixel(x, y).0[0] as f32;
                let local_mean = blurred.get_pixel(x, y).0[0] as f32;
                
                // Нормализация: (original - local_mean) + 128
                let normalized = ((original - local_mean) + 128.0)
                    .max(0.0)
                    .min(255.0) as u8;
                
                result.put_pixel(x, y, Luma([normalized]));
            }
        }
        
        result
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
