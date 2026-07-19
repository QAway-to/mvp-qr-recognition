import os
import random
import glob
import cv2
import numpy as np
from ultralytics import YOLO

MODEL_PATH = 'runs/obb_training/hardcore_local_v1/weights/best.pt'
VAL_DIR = 'dataset_v3_local/images/val'
OUTPUT_DIR = 'validation_results_epoch13'
NUM_SAMPLES = 10

def draw_obb(img, result, name):
    """Draw OBBs on image."""
    img_draw = img.copy()
    if result.obb is not None:
        for i, obb in enumerate(result.obb):
            corners = obb.xyxyxyxy[0].cpu().numpy().astype(int)
            conf = float(obb.conf[0])
            
            # Draw green polygon
            cv2.polylines(img_draw, [corners], True, (0, 255, 0), 2)
            
            # Draw corners with numbers
            for j, (x, y) in enumerate(corners):
                cv2.circle(img_draw, (x, y), 3, (0, 0, 255), -1)
                cv2.putText(img_draw, str(j), (x+5, y-5), cv2.FONT_HERSHEY_SIMPLEX, 0.4, (255, 0, 0), 1)
                
            # Draw confidence
            cx, cy = corners.mean(axis=0).astype(int)
            cv2.putText(img_draw, f'{conf:.2f}', (cx, cy), cv2.FONT_HERSHEY_SIMPLEX, 0.5, (0, 255, 255), 2)
            
    return img_draw

def main():
    if not os.path.exists(MODEL_PATH):
        print(f"Model not found: {MODEL_PATH}")
        return

    os.makedirs(OUTPUT_DIR, exist_ok=True)
    
    # Load model
    print(f"Loading {MODEL_PATH}...")
    model = YOLO(MODEL_PATH)
    
    # Get images
    # Get images recursively
    images = glob.glob(f"{VAL_DIR}/**/*.jpg", recursive=True)
    if not images:
        print(f"No images found in {VAL_DIR}")
        return
        
    # Select random samples
    samples = random.sample(images, min(len(images), NUM_SAMPLES))
    
    print(f"Processing {len(samples)} images...")
    
    for i, img_path in enumerate(samples):
        print(f"Processing {img_path}...")
        img = cv2.imread(img_path)
        if img is None:
            continue
            
        # Predict
        results = model.predict(img_path, conf=0.15, verbose=False) # Low conf to see even weak detections
        result = results[0]
        
        # Draw
        vis_img = draw_obb(img, result, os.path.basename(img_path))
        
        # Save
        save_path = os.path.join(OUTPUT_DIR, f"val_{i:02d}_{os.path.basename(img_path)}")
        cv2.imwrite(save_path, vis_img)
        
    print(f"\nDone! Results saved in '{OUTPUT_DIR}' folder.")

if __name__ == "__main__":
    main()
