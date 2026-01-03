use image::GrayImage;
use image::imageops::FilterType;
use tract_onnx::prelude::*;
use crate::detection::DetectedQR;
use crate::preprocessing::{ImageProcessor, ProcessingConfig};
use crate::geometry;

/// ML-based QR Detector using YOLOv8 (ONNX)
pub struct OnnxDetector {
    model: SimplePlan<TypedFact, Box<dyn TypedOp>, Graph<TypedFact, Box<dyn TypedOp>>>,
}

impl OnnxDetector {
    /// Load model from bytes (WASM compatible)
    pub fn load(model_bytes: &[u8]) -> anyhow::Result<Self> {
        let mut cursor = std::io::Cursor::new(model_bytes);
        let model = tract_onnx::onnx()
            .model_for_read(&mut cursor)?
            .with_input_fact(0, f32::fact([1, 3, 640, 640]).into())? // Force input shape
            .into_optimized()?
            .into_runnable()?;

        Ok(Self { model })
    }

    /// Detect QR codes in image
    pub fn detect(&self, img: &GrayImage) -> anyhow::Result<Vec<DetectedQR>> {
        let (orig_w, orig_h) = img.dimensions();
        const MODEL_SIZE: u32 = 640;

        // 1. Preprocessing: Resize to 640x640 (Stretch for speed/simplicity)
        // Convert Gray to RGB by triplicating channels (YOLO expects 3 channels)
        let resized = image::imageops::resize(img, MODEL_SIZE, MODEL_SIZE, FilterType::Triangle);
        
        let mut tensor_data = Vec::with_capacity((MODEL_SIZE * MODEL_SIZE * 3) as usize);
        
        // NCHW layout: (1, 3, 640, 640) -> Planar (RRR...GGG...BBB...)
        // Tract expects standard layout (check if RGB or BGR? usually RGB for ONNX from PyTorch)
        // We will fill 3 planes.
        
        let mut plane_r = Vec::with_capacity((MODEL_SIZE * MODEL_SIZE) as usize);
        let mut plane_g = Vec::with_capacity((MODEL_SIZE * MODEL_SIZE) as usize);
        let mut plane_b = Vec::with_capacity((MODEL_SIZE * MODEL_SIZE) as usize);

        for y in 0..MODEL_SIZE {
            for x in 0..MODEL_SIZE {
                let pixel = resized.get_pixel(x, y)[0] as f32 / 255.0;
                plane_r.push(pixel);
                plane_g.push(pixel);
                plane_b.push(pixel);
            }
        }
        
        tensor_data.extend_from_slice(&plane_r);
        tensor_data.extend_from_slice(&plane_g);
        tensor_data.extend_from_slice(&plane_b);

        let input_tensor = tract_ndarray::Array4::from_shape_vec(
            (1, 3, MODEL_SIZE as usize, MODEL_SIZE as usize),
            tensor_data,
        )?;

        // 2. Inference
        let tensor = Tensor::from(input_tensor);
        let result = self.model.run(tvec!(tensor.into()))?;
        
        // 3. Postprocessing
        // Output shape: (1, 4+nc, 8400) -> (1, 5, 8400) for 1 class
        let output = result[0].to_array_view::<f32>()?;
        let shape = output.shape(); // [1, nc+4, 8400]
        
        if shape.len() != 3 {
             return Ok(vec![]);
        }
        
        let num_classes = shape[1] - 4;
        let num_anchors = shape[2];
        
        let mut detections = Vec::new();
        let conf_threshold = 0.5;

        // Iterate over anchors
        for i in 0..num_anchors {
            // Find max class score
            let mut max_score = 0.0;
            let mut best_class = 0;
            
            for c in 0..num_classes {
                let score = output[[0, 4 + c, i]];
                if score > max_score {
                    max_score = score;
                    best_class = c;
                }
            }

            if max_score > conf_threshold {
                // Get box: cx, cy, w, h
                let cx = output[[0, 0, i]];
                let cy = output[[0, 1, i]];
                let w = output[[0, 2, i]];
                let h = output[[0, 3, i]];
                
                // Convert to x1, y1, x2, y2
                let x1 = cx - w / 2.0;
                let y1 = cy - h / 2.0;
                let x2 = cx + w / 2.0;
                let y2 = cy + h / 2.0;
                
                detections.push(BBox { x1, y1, x2, y2, score: max_score, class: best_class });
            }
        }
        
        // NMS
        let kept_boxes = nms(&detections, 0.45);
        
        // Map back to original image
        let mut qr_results = Vec::new();
        let scale_x = orig_w as f32 / MODEL_SIZE as f32;
        let scale_y = orig_h as f32 / MODEL_SIZE as f32;

        for bbox in kept_boxes {
            // Only class 0 usually implies QR if single class
            // Map coords
            let x = (bbox.x1 * scale_x).max(0.0) as u32;
            let y = (bbox.y1 * scale_y).max(0.0) as u32;
            let width = ((bbox.x2 - bbox.x1) * scale_x).max(1.0) as u32;
            let height = ((bbox.y2 - bbox.y1) * scale_y).max(1.0) as u32;
            
            // Check bounds
            if x + width > orig_w || y + height > orig_h {
                continue;
            }

            // Crop image
            let mut crop = image::imageops::crop_imm(img, x, y, width, height).to_image();
            let mut corners_abs = [
                (x, y), 
                (x + width, y), 
                (x + width, y + height), 
                (x, y + height)
            ];

            // "Best Practice": Refine corners and warp perspective
            // We create a temporary processor for this
            let processor = ImageProcessor::new(ProcessingConfig::default());
            
            if let Some(corners) = processor.find_corners(&crop) {
                // Determine output size (e.g. max side length of the quad)
                 let side_len = width.max(height); // Simple heuristic
                 
                 // Target is a square
                 let dst = [
                     nalgebra::Point2::new(0.0, 0.0),
                     nalgebra::Point2::new(side_len as f32, 0.0),
                     nalgebra::Point2::new(side_len as f32, side_len as f32),
                     nalgebra::Point2::new(0.0, side_len as f32),
                 ];
                 
                 if let Some(h) = geometry::find_homography(corners, dst) {
                     let warped = geometry::warp_perspective(&crop, &h, side_len, side_len);
                     crop = warped;
                     
                     // Update corners to be relative to the warped image?
                     // Actually DetectedQR.corners usually refers to location in *original* image.
                     // Mapping the refined corners back to original image is non-trivial if we only have relative corners.
                     // corners found are relative to `crop`.
                     // crop offset is (x, y).
                     
                     // Let's update corners_abs to reflect the refined corners in original image
                     let offset_x = x as f32;
                     let offset_y = y as f32;
                     
                     corners_abs = [
                         ((corners[0].x + offset_x) as u32, (corners[0].y + offset_y) as u32),
                         ((corners[1].x + offset_x) as u32, (corners[1].y + offset_y) as u32),
                         ((corners[2].x + offset_x) as u32, (corners[2].y + offset_y) as u32),
                         ((corners[3].x + offset_x) as u32, (corners[3].y + offset_y) as u32),
                     ];
                 }
            }

            qr_results.push(DetectedQR {
                bbox: [x, y, width, height],
                corners: corners_abs,
                image: crop,
                confidence: bbox.score,
            });
        }

        Ok(qr_results)
    }
}

#[derive(Clone, Copy, Debug)]
struct BBox {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
    score: f32,
    class: usize,
}

fn nms(boxes: &[BBox], iou_threshold: f32) -> Vec<BBox> {
    let mut sorted_boxes: Vec<_> = boxes.iter().collect();
    sorted_boxes.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    let mut kept = Vec::new();
    let mut suppress = vec![false; sorted_boxes.len()];

    for i in 0..sorted_boxes.len() {
        if suppress[i] { continue; }
        
        let bi = sorted_boxes[i];
        kept.push(BBox { ..*bi });

        for j in (i + 1)..sorted_boxes.len() {
            if suppress[j] { continue; }
            let bj = sorted_boxes[j];

            if iou(bi, bj) > iou_threshold {
                suppress[j] = true;
            }
        }
    }
    kept
}

fn iou(a: &BBox, b: &BBox) -> f32 {
    let x1 = a.x1.max(b.x1);
    let y1 = a.y1.max(b.y1);
    let x2 = a.x2.min(b.x2);
    let y2 = a.y2.min(b.y2);

    let w = (x2 - x1).max(0.0);
    let h = (y2 - y1).max(0.0);
    let inter = w * h;

    let area_a = (a.x2 - a.x1) * (a.y2 - a.y1);
    let area_b = (b.x2 - b.x1) * (b.y2 - b.y1);
    
    inter / (area_a + area_b - inter + 1e-6)
}
