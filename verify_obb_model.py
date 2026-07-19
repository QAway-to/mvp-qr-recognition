from ultralytics import YOLO
import cv2
import numpy as np
import os

# Paths
# Paths
MODEL_PATH = 'current_best.pt'
IMAGE_PATH = 'generated_dataset/text_rot_45.png'
OUTPUT_PATH = 'obb_verification_result.png'

def main():
    if not os.path.exists(MODEL_PATH):
        print(f"Error: Model not found at {MODEL_PATH}")
        return

    if not os.path.exists(IMAGE_PATH):
        print(f"Error: Image not found at {IMAGE_PATH}")
        return

    print("Loading model...")
    model = YOLO(MODEL_PATH)
    import ultralytics
    print(f"Ultralytics Version: {ultralytics.__version__}")
    
    print(f"Predicting on {IMAGE_PATH}...")
    
    try:
        results = model(IMAGE_PATH, verbose=True, task='obb')
    except Exception as e:
        import traceback
        traceback.print_exc()
        print(f"CRITICAL ERROR during inference: {e}")
        return
    
    # Load original image for custom drawing
    
    # Load original image for custom drawing
    img = cv2.imread(IMAGE_PATH)
    
    result = results[0]
    
    print(f"Result keys: {result.keys()}")
    if result.boxes is not None:
        print(f"Standard Boxes detected: {len(result.boxes)}")
    if result.obb is not None:
        print(f"OBB detected: {len(result.obb)}")

    if result.obb is None:
        print("No OBB detections found! (Is this a standard detection model?)")
        if result.boxes:
             print("Falling back to standard boxes visualization (partial verify)")
             for box in result.boxes:
                 print(f"Box: {box.xyxy}")
        return

    print(f"Found {len(result.obb)} OBB detections.")
    
    for i, obb in enumerate(result.obb):
        # Extract OBB parameters
        # rbbox: [cx, cy, w, h, angle] or xyxyxyxy
        # ultralytics .obb return xyxyxyxy (4 corners)
        
        corners = obb.xyxyxyxy[0].cpu().numpy().astype(int)
        conf = float(obb.conf[0])
        cls = int(obb.cls[0])
        
        print(f"Detection #{i}: Conf={conf:.2f}")
        
        # Draw rotated rectangle
        cv2.drawContours(img, [corners], 0, (0, 255, 0), 2)
        
        # Calculate angle and upright crop
        rect = cv2.minAreaRect(corners)
        (center, size, angle) = rect
        box = cv2.boxPoints(rect)
        box = np.int0(box)
        
        # Draw minAreaRect (red) to compare
        cv2.drawContours(img, [box], 0, (0, 0, 255), 1)
        
        print(f"  Rotated Rect Angle: {angle:.2f}")
        
    print(f"Saving result to {OUTPUT_PATH}")
    cv2.imwrite(OUTPUT_PATH, img)
    print("Done.")

if __name__ == "__main__":
    main()
