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
    
    /// Адаптивная бинаризация (Adaptive Thresholding)
    /// Uses Integral Image (Summed Area Table) for O(1) window sum calculation.
    /// This removes glare and uneven lighting, creating a clean binary mask.
    pub fn adaptive_threshold(&self, img: &GrayImage) -> GrayImage {
        let (width, height) = img.dimensions();
        let mut output = GrayImage::new(width, height);
        
        if width == 0 || height == 0 {
            return output;
        }

        // 1. Calculate Integral Image (Summed Area Table)
        // We use u32 to prevent overflow (frame max 1000x1000 * 255 < u32::MAX)
        let mut integral = vec![0u32; (width * height) as usize];

        // First row
        let mut row_sum = 0;
        for x in 0..width {
            row_sum += img.get_pixel(x, 0).0[0] as u32;
            integral[x as usize] = row_sum;
        }

        // Subsequent rows
        for y in 1..height {
            row_sum = 0;
            for x in 0..width {
                row_sum += img.get_pixel(x, y).0[0] as u32;
                let idx = (y * width + x) as usize;
                let top_idx = ((y - 1) * width + x) as usize;
                integral[idx] = integral[top_idx] + row_sum;
            }
        }

        // 2. Apply Threshold
        // Configurable window size (must be odd)
        let s = self.config.block_size / 2; // radius
        let t = 15; // Constant subtraction (makes it robust to noise) -> similar to C in cv2

        for y in 0..height {
            for x in 0..width {
                // Calculate window boundaries
                let x1 = x.saturating_sub(s);
                let x2 = (x + s).min(width - 1);
                let y1 = y.saturating_sub(s);
                let y2 = (y + s).min(height - 1);

                // Calculate sum using Integral Image
                // Sum = I(x2, y2) - I(x2, y1-1) - I(x1-1, y2) + I(x1-1, y1-1)
                // Use i64 to prevent temporary underflow (e.g. I(br) - I(tr) - I(bl) might be negative before adding I(tl))
                
                let idx_br = (y2 * width + x2) as usize;
                let count = (x2 - x1 + 1) * (y2 - y1 + 1);
                let mut sum: i64 = integral[idx_br] as i64;

                if y1 > 0 {
                    let idx_tr = ((y1 - 1) * width + x2) as usize;
                    sum -= integral[idx_tr] as i64;
                }
                
                if x1 > 0 {
                    let idx_bl = (y2 * width + (x1 - 1)) as usize;
                    sum -= integral[idx_bl] as i64;
                }
                
                if x1 > 0 && y1 > 0 {
                    let idx_tl = ((y1 - 1) * width + (x1 - 1)) as usize;
                    sum += integral[idx_tl] as i64;
                }

                let mean = (sum as u32) / count;
                let pixel = img.get_pixel(x, y).0[0] as u32;

                // If pixel is significantly darker than local mean, it's black (foreground)
                if pixel < mean.saturating_sub(t) {
                    output.put_pixel(x, y, Luma([0]));
                } else {
                    output.put_pixel(x, y, Luma([255]));
                }
            }
        }

        output
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
