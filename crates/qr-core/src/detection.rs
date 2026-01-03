//! Модуль обнаружения QR-кодов
//!
//! Реализация алгоритмического обнаружения QR-кодов через finder patterns

use image::{GrayImage, Luma};
use serde::{Deserialize, Serialize};

/// Конфигурация детектора
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectorConfig {
    /// Минимальный размер QR-кода в пикселях
    pub min_size: u32,
    /// Максимальный размер QR-кода в пикселях  
    pub max_size: u32,
    /// Порог бинаризации (0-255)
    pub threshold: u8,
    /// Допуск отклонения соотношения 1:1:3:1:1
    pub ratio_tolerance: f32,
}

impl Default for DetectorConfig {
    fn default() -> Self {
        Self {
            min_size: 20,
            max_size: 2000,
            threshold: 128,
            ratio_tolerance: 0.5,
        }
    }
}

/// Обнаруженный QR-код
#[derive(Debug, Clone)]
pub struct DetectedQR {
    /// Bounding box [x, y, width, height]
    pub bbox: [u32; 4],
    /// Углы QR-кода [top-left, top-right, bottom-right, bottom-left]
    pub corners: [(u32, u32); 4],
    /// Вырезанное изображение QR-кода
    pub image: GrayImage,
    /// Уверенность обнаружения (0.0 - 1.0)
    pub confidence: f32,
}

/// Finder pattern QR-кода
#[derive(Debug, Clone)]
struct FinderPattern {
    center_x: u32,
    center_y: u32,
    module_size: f32,
}

/// Детектор QR-кодов
pub struct QRDetector {
    config: DetectorConfig,
}

impl QRDetector {
    /// Создание детектора
    pub fn new(config: DetectorConfig) -> Self {
        Self { config }
    }
    
    /// Обнаружение всех QR-кодов на изображении
    pub fn detect(&self, img: &GrayImage) -> Vec<DetectedQR> {
        let mut results = Vec::new();
        
        // 1. Поиск finder patterns
        let patterns = self.find_finder_patterns(img);
        
        // 2. Группировка паттернов в тройки (3 finder pattern = 1 QR)
        let groups = self.group_patterns(&patterns);
        
        // 3. Для каждой группы создаём DetectedQR
        for group in groups {
            if let Some(detected) = self.extract_qr(img, &group) {
                results.push(detected);
            }
        }
        
        // Если поиск по паттернам не дал результатов, возвращаем всё изображение
        if results.is_empty() {
            let (width, height) = img.dimensions();
            results.push(DetectedQR {
                bbox: [0, 0, width, height],
                corners: [(0, 0), (width, 0), (width, height), (0, height)],
                image: img.clone(),
                confidence: 0.5,
            });
        }
        
        results
    }
    
    /// Поиск finder patterns (паттерны 1:1:3:1:1)
    fn find_finder_patterns(&self, img: &GrayImage) -> Vec<FinderPattern> {
        let mut patterns = Vec::new();
        let (width, height) = img.dimensions();
        let threshold = self.config.threshold;
        
        // Сканируем горизонтальные линии
        for y in 0..height {
            let mut state_count = [0u32; 5];
            let mut current_state = 0usize;
            
            for x in 0..width {
                let pixel = img.get_pixel(x, y).0[0];
                let is_black = pixel < threshold;
                
                // Переключение состояния
                if is_black {
                    // Чёрный пиксель
                    if current_state % 2 == 1 {
                        // Переход white -> black
                        current_state += 1;
                        if current_state >= 5 {
                            // Проверяем паттерн
                            if self.check_ratio(&state_count) {
                                let total_width: u32 = state_count.iter().sum();
                                let center_x = x - total_width / 2;
                                
                                // Верификация по вертикали
                                if self.verify_vertical(img, center_x, y, &state_count) {
                                    let module_size = total_width as f32 / 7.0;
                                    patterns.push(FinderPattern {
                                        center_x,
                                        center_y: y,
                                        module_size,
                                    });
                                }
                            }
                            
                            // Сдвиг состояний
                            state_count[0] = state_count[2];
                            state_count[1] = state_count[3];
                            state_count[2] = state_count[4];
                            state_count[3] = 1;
                            state_count[4] = 0;
                            current_state = 3;
                        }
                    }
                    state_count[current_state] += 1;
                } else {
                    // Белый пиксель
                    if current_state % 2 == 0 {
                        // Переход black -> white
                        current_state += 1;
                        if current_state >= 5 {
                            current_state = 4;
                        }
                    }
                    if current_state < 5 {
                        state_count[current_state] += 1;
                    }
                }
            }
        }
        
        // Удаление дубликатов
        self.merge_patterns(patterns)
    }
    
    /// Проверка соотношения 1:1:3:1:1
    fn check_ratio(&self, counts: &[u32; 5]) -> bool {
        let total: u32 = counts.iter().sum();
        if total < 7 {
            return false;
        }
        
        let module_size = total as f32 / 7.0;
        let tolerance = module_size * self.config.ratio_tolerance;
        
        // Проверяем каждый сегмент
        let expected = [1.0, 1.0, 3.0, 1.0, 1.0];
        
        for (i, &count) in counts.iter().enumerate() {
            let expected_size = expected[i] * module_size;
            if (count as f32 - expected_size).abs() > tolerance {
                return false;
            }
        }
        
        true
    }
    
    /// Верификация паттерна по вертикали
    fn verify_vertical(&self, img: &GrayImage, center_x: u32, center_y: u32, h_counts: &[u32; 5]) -> bool {
        let (_, height) = img.dimensions();
        let threshold = self.config.threshold;
        
        let mut v_counts = [0u32; 5];
        let total_h: u32 = h_counts.iter().sum();
        let check_range = total_h / 2;
        
        // Сканируем вверх и вниз от центра
        let start_y = center_y.saturating_sub(check_range);
        let end_y = (center_y + check_range).min(height - 1);
        
        let mut state = 0usize;
        for y in start_y..=end_y {
            let pixel = img.get_pixel(center_x, y).0[0];
            let is_black = pixel < threshold;
            
            let expected_black = state % 2 == 0;
            
            if is_black == expected_black {
                v_counts[state] += 1;
            } else {
                state += 1;
                if state >= 5 {
                    break;
                }
                v_counts[state] = 1;
            }
        }
        
        self.check_ratio(&v_counts)
    }
    
    /// Объединение близких паттернов
    fn merge_patterns(&self, patterns: Vec<FinderPattern>) -> Vec<FinderPattern> {
        if patterns.is_empty() {
            return patterns;
        }
        
        let mut merged = Vec::new();
        let mut used = vec![false; patterns.len()];
        
        for (i, p1) in patterns.iter().enumerate() {
            if used[i] {
                continue;
            }
            
            let mut sum_x = p1.center_x as f32;
            let mut sum_y = p1.center_y as f32;
            let mut sum_size = p1.module_size;
            let mut count = 1.0f32;
            
            for (j, p2) in patterns.iter().enumerate().skip(i + 1) {
                if used[j] {
                    continue;
                }
                
                let dist = ((p1.center_x as f32 - p2.center_x as f32).powi(2) +
                           (p1.center_y as f32 - p2.center_y as f32).powi(2))
                    .sqrt();
                
                // Объединяем если расстояние меньше 2 размеров модуля
                if dist < p1.module_size * 2.0 {
                    sum_x += p2.center_x as f32;
                    sum_y += p2.center_y as f32;
                    sum_size += p2.module_size;
                    count += 1.0;
                    used[j] = true;
                }
            }
            
            merged.push(FinderPattern {
                center_x: (sum_x / count) as u32,
                center_y: (sum_y / count) as u32,
                module_size: sum_size / count,
            });
            used[i] = true;
        }
        
        merged
    }
    
    /// Группировка паттернов в тройки
    fn group_patterns(&self, patterns: &[FinderPattern]) -> Vec<[FinderPattern; 3]> {
        let mut groups = Vec::new();
        
        if patterns.len() < 3 {
            return groups;
        }
        
        // Простая эвристика: берём все комбинации из 3 паттернов
        // и проверяем, образуют ли они правильный угол
        for i in 0..patterns.len() {
            for j in (i + 1)..patterns.len() {
                for k in (j + 1)..patterns.len() {
                    let p1 = &patterns[i];
                    let p2 = &patterns[j];
                    let p3 = &patterns[k];
                    
                    if self.is_valid_qr_group(p1, p2, p3) {
                        groups.push([p1.clone(), p2.clone(), p3.clone()]);
                    }
                }
            }
        }
        
        groups
    }
    
    /// Проверка, образуют ли 3 паттерна валидный QR
    fn is_valid_qr_group(&self, p1: &FinderPattern, p2: &FinderPattern, p3: &FinderPattern) -> bool {
        // Размеры модулей должны быть примерно одинаковыми
        let sizes = [p1.module_size, p2.module_size, p3.module_size];
        let avg_size = sizes.iter().sum::<f32>() / 3.0;
        
        for &size in &sizes {
            if (size - avg_size).abs() > avg_size * 0.5 {
                return false;
            }
        }
        
        // Расстояния должны быть примерно равны (квадрат)
        let d12 = self.distance(p1, p2);
        let d23 = self.distance(p2, p3);
        let d13 = self.distance(p1, p3);
        
        let mut distances = [d12, d23, d13];
        distances.sort_by(|a, b| a.partial_cmp(b).unwrap());
        
        // Два меньших расстояния должны быть примерно равны (стороны)
        // Большее расстояние - диагональ
        let side1 = distances[0];
        let side2 = distances[1];
        let diagonal = distances[2];
        
        // Диагональ должна быть примерно √2 от сторон
        let expected_diagonal = side1 * 1.414;
        
        (side1 - side2).abs() < side1 * 0.3 &&
        (diagonal - expected_diagonal).abs() < expected_diagonal * 0.3
    }
    
    /// Расстояние между двумя паттернами
    fn distance(&self, p1: &FinderPattern, p2: &FinderPattern) -> f32 {
        let dx = p1.center_x as f32 - p2.center_x as f32;
        let dy = p1.center_y as f32 - p2.center_y as f32;
        (dx * dx + dy * dy).sqrt()
    }
    
    /// Извлечение QR из группы паттернов
    fn extract_qr(&self, img: &GrayImage, group: &[FinderPattern; 3]) -> Option<DetectedQR> {
        // Находим bounding box
        let min_x = group.iter().map(|p| p.center_x).min()? as i32 - 20;
        let max_x = group.iter().map(|p| p.center_x).max()? as i32 + 20;
        let min_y = group.iter().map(|p| p.center_y).min()? as i32 - 20;
        let max_y = group.iter().map(|p| p.center_y).max()? as i32 + 20;
        
        let (width, height) = img.dimensions();
        
        let x = min_x.max(0) as u32;
        let y = min_y.max(0) as u32;
        let w = ((max_x - min_x) as u32).min(width - x);
        let h = ((max_y - min_y) as u32).min(height - y);
        
        // Проверка размера
        if w < self.config.min_size || h < self.config.min_size ||
           w > self.config.max_size || h > self.config.max_size {
            return None;
        }
        
        // Вырезаем изображение
        let cropped = image::imageops::crop_imm(img, x, y, w, h).to_image();
        
        Some(DetectedQR {
            bbox: [x, y, w, h],
            corners: [
                (x, y),
                (x + w, y),
                (x + w, y + h),
                (x, y + h),
            ],
            image: cropped,
            confidence: 0.8,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_detector_creation() {
        let config = DetectorConfig::default();
        let _detector = QRDetector::new(config);
    }
    
    #[test]
    fn test_ratio_check() {
        let detector = QRDetector::new(DetectorConfig::default());
        
        // Идеальное соотношение 1:1:3:1:1
        assert!(detector.check_ratio(&[10, 10, 30, 10, 10]));
        
        // С небольшим отклонением
        assert!(detector.check_ratio(&[9, 11, 29, 10, 11]));
        
        // Неправильное соотношение
        assert!(!detector.check_ratio(&[10, 10, 10, 10, 10]));
    }
}
