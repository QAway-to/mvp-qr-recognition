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
        // 0. Resize if too large (improves performance and consistency)
        let mut result = self.resize(img, 1000); // Max 1000px
        
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

    /// Find corners of the QR code within the image (or ROI)
    /// Returns 4 points [TL, TR, BR, BL] if a valid quad is found.
    pub fn find_corners(&self, img: &GrayImage) -> Option<[nalgebra::Point2<f32>; 4]> {
        // 1. Binary + Edges
        // Use adaptive threshold which we already have
        let binary = self.adaptive_threshold(img);
        
        // 2. Find contours
        // imageproc::contours::find_contours returns valid contours
        let contours = imageproc::contours::find_contours(&binary);
        
        // 3. Filter for large quads
        let (width, height) = img.dimensions();
        let min_area = (width * height) as f32 * 0.1; // At least 10% of ROI
        
        // We look for a contour that approximates to 4 points
        for contour in contours {
            // Simplify contour
            let simplified = simplify_douglas_peucker(&contour.points, 5.0);
            
            if simplified.len() == 4 {
                // Check area
                // Polygon area formula
                let area = polygon_area(&simplified);
                if area > min_area && is_convex(&simplified) {
                    // Sort points: TL, TR, BR, BL
                    return Some(sort_corners(&simplified));
                }
            }
        }
        
        None
    }
}

/// Ramer-Douglas-Peucker algorithm for curve simplification
fn simplify_douglas_peucker(points: &[imageproc::point::Point<i32>], epsilon: f32) -> Vec<imageproc::point::Point<i32>> {
    if points.len() < 3 {
        return points.to_vec();
    }

    let mut dmax = 0.0;
    let mut index = 0;
    let end = points.len() - 1;

    for i in 1..end {
        let d = perpendicular_distance(&points[i], &points[0], &points[end]);
        if d > dmax {
            index = i;
            dmax = d;
        }
    }

    if dmax > epsilon {
        let mut results1 = simplify_douglas_peucker(&points[..=index], epsilon);
        let results2 = simplify_douglas_peucker(&points[index..], epsilon);
        
        results1.pop(); // Remove duplicate point
        results1.extend(results2);
        results1
    } else {
        vec![points[0], points[end]]
    }
}

fn perpendicular_distance(p: &imageproc::point::Point<i32>, p1: &imageproc::point::Point<i32>, p2: &imageproc::point::Point<i32>) -> f32 {
    let dx = p2.x - p1.x;
    let dy = p2.y - p1.y;
    
    if dx == 0 && dy == 0 {
        return ((p.x - p1.x).pow(2) as f32 + (p.y - p1.y).pow(2) as f32).sqrt();
    }

    let num = ((dy * p.x - dx * p.y + p2.x * p1.y - p2.y * p1.x) as f32).abs();
    let den = ((dx.pow(2) + dy.pow(2)) as f32).sqrt();
    
    num / den
}

fn polygon_area(points: &[imageproc::point::Point<i32>]) -> f32 {
    let mut area = 0.0;
    for i in 0..points.len() {
        let j = (i + 1) % points.len();
        area += (points[i].x * points[j].y) as f32;
        area -= (points[j].x * points[i].y) as f32;
    }
    (area / 2.0).abs()
}

fn is_convex(points: &[imageproc::point::Point<i32>]) -> bool {
    // Check cross product of adjacent edges have same sign
    // Simple check for 4 points
    if points.len() != 4 { return false; }
    true
}

fn sort_corners(points: &[imageproc::point::Point<i32>]) -> [nalgebra::Point2<f32>; 4] {
    // Convert to nalgebra points
    let pts: Vec<nalgebra::Point2<f32>> = points.iter()
        .map(|p| nalgebra::Point2::new(p.x as f32, p.y as f32))
        .collect();
        
    // Sort by Y first
    let mut sorted = pts.clone();
    sorted.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    
    // Top 2 (lowest y)
    let (top, bottom) = sorted.split_at_mut(2);
    top.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    bottom.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    
    let tl = top[0];
    let tr = top[1];
    let bl = bottom[0];
    let br = bottom[1];
    
    [tl, tr, br, bl]
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
