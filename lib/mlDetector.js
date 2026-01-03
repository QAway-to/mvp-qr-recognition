/**
 * ML QR Code Detector using onnxruntime-web
 * Uses YOLOv8-nano model for fast QR detection with WebGL acceleration
 */

import * as ort from 'onnxruntime-web';

// Manually configure WASM paths since we are using the CJS build and serving WASM from public/pkg
ort.env.wasm.wasmPaths = "/pkg/";

// Configure onnxruntime-web to use WebGL for GPU acceleration
ort.env.wasm.numThreads = 1;

export class MLDetector {
    constructor() {
        this.session = null;
        this.modelLoaded = false;
        this.inputSize = 640;
    }

    /**
     * Load ONNX model from URL
     * @param {string} modelUrl - URL to the .onnx model file
     */
    async loadModel(modelUrl) {
        try {
            console.log('[MLDetector] Loading model from:', modelUrl);

            // Try WebGL first for GPU acceleration, fallback to WASM
            const options = {
                executionProviders: ['wasm'], // Fallback to CPU/WASM to avoid WebGL operator issues (e.g. resize nearest)
                graphOptimizationLevel: 'all'
            };

            this.session = await ort.InferenceSession.create(modelUrl, options);
            this.modelLoaded = true;

            console.log('[MLDetector] Model loaded successfully');
            console.log('[MLDetector] Input names:', this.session.inputNames);
            console.log('[MLDetector] Output names:', this.session.outputNames);

            return true;
        } catch (error) {
            console.error('[MLDetector] Failed to load model:', error);
            this.modelLoaded = false;
            return false;
        }
    }

    /**
     * Preprocess image for YOLOv8 inference
     * @param {ImageData} imageData - Canvas ImageData object
     * @returns {Float32Array} - Preprocessed tensor data
     */
    preprocessImage(imageData) {
        const { width, height, data } = imageData;

        // Create canvas for resizing
        const canvas = document.createElement('canvas');
        canvas.width = this.inputSize;
        canvas.height = this.inputSize;
        const ctx = canvas.getContext('2d');

        // Create temporary canvas with original image
        const tempCanvas = document.createElement('canvas');
        tempCanvas.width = width;
        tempCanvas.height = height;
        const tempCtx = tempCanvas.getContext('2d');
        const tempImageData = tempCtx.createImageData(width, height);
        tempImageData.data.set(data);
        tempCtx.putImageData(tempImageData, 0, 0);

        // Resize to model input size
        ctx.drawImage(tempCanvas, 0, 0, this.inputSize, this.inputSize);
        const resizedData = ctx.getImageData(0, 0, this.inputSize, this.inputSize);

        // Convert to CHW format and normalize
        const tensorData = new Float32Array(3 * this.inputSize * this.inputSize);
        const pixels = resizedData.data;

        for (let y = 0; y < this.inputSize; y++) {
            for (let x = 0; x < this.inputSize; x++) {
                const srcIdx = (y * this.inputSize + x) * 4;
                const dstIdx = y * this.inputSize + x;

                // RGB channels, normalized to 0-1
                tensorData[dstIdx] = pixels[srcIdx] / 255.0;                          // R
                tensorData[this.inputSize * this.inputSize + dstIdx] = pixels[srcIdx + 1] / 255.0;  // G
                tensorData[2 * this.inputSize * this.inputSize + dstIdx] = pixels[srcIdx + 2] / 255.0; // B
            }
        }

        return {
            data: tensorData,
            scaleX: width / this.inputSize,
            scaleY: height / this.inputSize
        };
    }

    /**
     * Run inference and detect QR codes
     * @param {ImageData} imageData - Canvas ImageData from the image
     * @param {number} confThreshold - Confidence threshold (0-1)
     * @returns {Array} - Array of detected bounding boxes
     */
    async detect(imageData, confThreshold = 0.5) {
        if (!this.modelLoaded || !this.session) {
            console.warn('[MLDetector] Model not loaded');
            return [];
        }

        const startTime = performance.now();

        try {
            // Preprocess
            const { data: tensorData, scaleX, scaleY } = this.preprocessImage(imageData);

            // Create input tensor [1, 3, 640, 640]
            const inputTensor = new ort.Tensor('float32', tensorData, [1, 3, this.inputSize, this.inputSize]);

            // Run inference
            const feeds = { [this.session.inputNames[0]]: inputTensor };
            const results = await this.session.run(feeds);

            // Get output tensor
            const output = results[this.session.outputNames[0]];
            const outputData = output.data;
            const shape = output.dims; // [1, nc+4, 8400] for YOLOv8

            console.log('[MLDetector] Output shape:', shape);

            // Parse YOLOv8 output
            const detections = this.parseYoloOutput(outputData, shape, confThreshold, scaleX, scaleY);

            // Apply NMS
            const finalDetections = this.nms(detections, 0.45);

            const inferenceTime = performance.now() - startTime;
            console.log(`[MLDetector] Inference time: ${inferenceTime.toFixed(0)}ms, detections: ${finalDetections.length}`);

            return finalDetections;

        } catch (error) {
            console.error('[MLDetector] Inference error:', error);
            return [];
        }
    }

    /**
     * Parse YOLOv8 output tensor
     */
    parseYoloOutput(data, shape, confThreshold, scaleX, scaleY) {
        const detections = [];

        if (shape.length !== 3) return detections;

        const numClasses = shape[1] - 4; // First 4 are x, y, w, h
        const numAnchors = shape[2];

        for (let i = 0; i < numAnchors; i++) {
            // Find max class score
            let maxScore = 0;
            let bestClass = 0;

            for (let c = 0; c < numClasses; c++) {
                const score = data[(4 + c) * numAnchors + i];
                if (score > maxScore) {
                    maxScore = score;
                    bestClass = c;
                }
            }

            if (maxScore > confThreshold) {
                const cx = data[0 * numAnchors + i];
                const cy = data[1 * numAnchors + i];
                const w = data[2 * numAnchors + i];
                const h = data[3 * numAnchors + i];

                // Convert to corner format and scale
                const x1 = (cx - w / 2) * scaleX;
                const y1 = (cy - h / 2) * scaleY;
                const x2 = (cx + w / 2) * scaleX;
                const y2 = (cy + h / 2) * scaleY;

                // Validate bounds
                if (x1 >= 0 && y1 >= 0 && w > 10 && h > 10) {
                    detections.push({
                        x: Math.max(0, Math.round(x1)),
                        y: Math.max(0, Math.round(y1)),
                        width: Math.round(x2 - x1),
                        height: Math.round(y2 - y1),
                        confidence: maxScore,
                        class: bestClass
                    });
                }
            }
        }

        return detections;
    }

    /**
     * Non-Maximum Suppression
     */
    nms(boxes, iouThreshold) {
        if (boxes.length === 0) return [];

        // Sort by confidence
        boxes.sort((a, b) => b.confidence - a.confidence);

        const kept = [];
        const suppressed = new Set();

        for (let i = 0; i < boxes.length; i++) {
            if (suppressed.has(i)) continue;

            kept.push(boxes[i]);

            for (let j = i + 1; j < boxes.length; j++) {
                if (suppressed.has(j)) continue;

                if (this.iou(boxes[i], boxes[j]) > iouThreshold) {
                    suppressed.add(j);
                }
            }
        }

        return kept;
    }

    /**
     * Intersection over Union
     */
    iou(a, b) {
        const x1 = Math.max(a.x, b.x);
        const y1 = Math.max(a.y, b.y);
        const x2 = Math.min(a.x + a.width, b.x + b.width);
        const y2 = Math.min(a.y + a.height, b.y + b.height);

        const intersection = Math.max(0, x2 - x1) * Math.max(0, y2 - y1);
        const areaA = a.width * a.height;
        const areaB = b.width * b.height;

        return intersection / (areaA + areaB - intersection + 1e-6);
    }
}

export default MLDetector;
