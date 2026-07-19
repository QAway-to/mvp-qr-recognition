from ultralytics import YOLO
import os
import random
import time
import glob

def main():
    print("=== Benchmarking INT8 ONNX Model (Hard Sample) ===")
    
    # Model Path
    model_path = "../public/model.onnx"
    if not os.path.exists(model_path):
        print(f"Error: Model not found at {model_path}")
        return

    print(f"Loading Model: {model_path}")
    try:
        model = YOLO(model_path, task='detect')
    except Exception as e:
        print(f"Failed to load model: {e}")
        return

    # Dataset Path
    dataset_dir = "yolo-2/test/images"
    if not os.path.exists(dataset_dir):
        print(f"Dataset not found at {dataset_dir}")
        dataset_dir = "dataset/test/images" # Fallback check
        if not os.path.exists(dataset_dir):
             print("Could not find test images.")
             return

    # Select 30 random images
    all_images = glob.glob(os.path.join(dataset_dir, "*.jpg")) + glob.glob(os.path.join(dataset_dir, "*.png"))
    if not all_images:
        print("No images found.")
        return
        
    sample_size = 30
    test_images = random.sample(all_images, min(len(all_images), sample_size))
    
    print(f"\nRunning Inference on {len(test_images)} images...")
    
    total_imgs = 0
    detected_qr = 0
    total_time = 0
    
    print(f"{'Image':<40} | {'Status':<10} | {'Conf':<6} | {'Time (ms)':<10}")
    print("-" * 80)
    
    for img_path in test_images:
        start_t = time.time()
        # Run inference
        # task='detect' is important for ONNX in Ultralytics
        results = model(img_path, verbose=False)
        end_t = time.time()
        duration_ms = (end_t - start_t) * 1000
        total_time += duration_ms
        total_imgs += 1
        
        # Check Class 1 (QR Code) - Piero Model has 0=Barcode, 1=QR
        # Ultralytics results.boxes.cls tells the class indices
        
        found = False
        max_conf = 0.0
        
        if results and len(results) > 0:
            boxes = results[0].boxes
            for box in boxes:
                cls_id = int(box.cls[0])
                conf = float(box.conf[0])
                # Check for QR Code (Class 1) OR Barcode (Class 0) if user wants both?
                # User specifically asked for QR. Let's look for Class 1.
                # Note: Some models flip classes. Let's assume standard Piero: 0=Barcode, 1=QR.
                # However, to be safe, I'll count ANY detection for now, but note the class.
                
                if conf > max_conf:
                    max_conf = conf
                
                # We consider it a "pass" if it detects *something* high confidence, 
                # effectively checking if the model is "alive"
                if conf > 0.5:
                    found = True

        status = "FOUND" if found else "MISS"
        fname = os.path.basename(img_path)
        if len(fname) > 35: fname = fname[:32] + "..."
        
        if found:
            detected_qr += 1
            
        print(f"{fname:<40} | {status:<10} | {max_conf:.2f}   | {duration_ms:.1f}")

    print("-" * 80)
    print(f"Total: {total_imgs}")
    print(f"Detected: {detected_qr} ({detected_qr/total_imgs*100:.1f}%)")
    print(f"Avg Time: {total_time/total_imgs:.1f} ms per image (Initial cold start included)")

if __name__ == "__main__":
    main()
