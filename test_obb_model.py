"""
OBB Model Verification Script
Tests the newly trained OBB model on:
1. Challenging rotated images (text_rot_45.png, etc.)
2. Validation set images
3. Attempts to decode detected QR codes after de-rotation
"""

from ultralytics import YOLO
import cv2
import numpy as np
import os
from pyzbar.pyzbar import decode as pyzbar_decode
import glob

# Configuration
MODEL_PATH = 'best_obb_v2.pt'
TEST_IMAGES = [
    'generated_dataset/text_rot_45.png',
    'generated_dataset/text_rot_30.png',
    'generated_dataset/text_rot_0.png',
]
VAL_DIR = 'generated_obb_dataset_v2/images/val'
CONF_THRESHOLD = 0.3

def get_rotation_angle(obb_result):
    """Extract rotation angle from OBB result."""
    if obb_result.obb is None or len(obb_result.obb) == 0:
        return None, None
    
    # Get first detection (highest confidence)
    obb = obb_result.obb[0]
    
    # xywhr format: center_x, center_y, width, height, rotation (radians)
    xywhr = obb.xywhr[0].cpu().numpy()
    angle_rad = xywhr[4]
    angle_deg = np.degrees(angle_rad)
    
    # Get corners for cropping
    corners = obb.xyxyxyxy[0].cpu().numpy().astype(int)
    confidence = float(obb.conf[0])
    
    return {
        'angle_deg': angle_deg,
        'angle_rad': angle_rad,
        'corners': corners,
        'confidence': confidence,
        'xywhr': xywhr,
    }

def order_corners(pts):
    """Order corners: top-left, top-right, bottom-right, bottom-left."""
    # Sort by y first, then by x
    pts = pts[np.argsort(pts[:, 1])]  # Sort by y
    
    # Top two points (smallest y)
    top = pts[:2]
    top = top[np.argsort(top[:, 0])]  # Sort by x: left, right
    
    # Bottom two points (largest y)
    bottom = pts[2:]
    bottom = bottom[np.argsort(bottom[:, 0])]  # Sort by x: left, right
    
    # Order: TL, TR, BR, BL
    return np.array([top[0], top[1], bottom[1], bottom[0]], dtype=np.float32)

def de_rotate_crop(image, detection):
    """Crop and de-rotate the detected QR region using OBB corners."""
    corners = detection['corners'].astype(np.float32)
    
    # Clip corners to image bounds to handle out-of-frame predictions
    if image is not None:
        h, w = image.shape[:2]
        corners[:, 0] = np.clip(corners[:, 0], 0, w)
        corners[:, 1] = np.clip(corners[:, 1], 0, h)
    
    # Sort corners to consistent order: TL, TR, BR, BL
    corners = order_corners(corners)
    
    # Calculate width and height from the corners
    width = int(np.linalg.norm(corners[0] - corners[1]))
    height = int(np.linalg.norm(corners[1] - corners[2]))
    
    if width == 0 or height == 0:
        return None
    
    # Destination points: axis-aligned rectangle
    dst_pts = np.array([
        [0, 0],
        [width - 1, 0],
        [width - 1, height - 1],
        [0, height - 1]
    ], dtype=np.float32)
    
    # Get perspective transform matrix
    M = cv2.getPerspectiveTransform(corners, dst_pts)
    
    # Apply warp
    warped = cv2.warpPerspective(image, M, (width, height))
    
    return warped

def try_decode(image):
    """Attempt to decode QR code using pyzbar."""
    if image is None or image.size == 0:
        return None
    
    # Try original
    decoded = pyzbar_decode(image)
    if decoded:
        return decoded[0].data.decode('utf-8')
    
    # Try grayscale
    if len(image.shape) == 3:
        gray = cv2.cvtColor(image, cv2.COLOR_BGR2GRAY)
        decoded = pyzbar_decode(gray)
        if decoded:
            return decoded[0].data.decode('utf-8')
    
    # Try inverted
    inverted = cv2.bitwise_not(image if len(image.shape) == 2 else gray)
    decoded = pyzbar_decode(inverted)
    if decoded:
        return decoded[0].data.decode('utf-8')
    
    # Try thresholded
    _, thresh = cv2.threshold(gray if len(image.shape) == 3 else image, 128, 255, cv2.THRESH_BINARY)
    decoded = pyzbar_decode(thresh)
    if decoded:
        return decoded[0].data.decode('utf-8')
    
    return None

def test_image(model, image_path):
    """Test a single image."""
    print(f"\n{'='*50}")
    print(f"Testing: {image_path}")
    
    if not os.path.exists(image_path):
        print(f"  ❌ File not found")
        return False
    
    # Load image
    img = cv2.imread(image_path)
    if img is None:
        print(f"  ❌ Failed to load image")
        return False
    
    # Run inference
    results = model.predict(image_path, conf=CONF_THRESHOLD, verbose=False)
    result = results[0]
    
    if result.obb is None or len(result.obb) == 0:
        print(f"  ❌ No OBB detections")
        return False
    
    print(f"  ✅ Found {len(result.obb)} detection(s)")
    
    # Process first detection
    detection = get_rotation_angle(result)
    if detection is None:
        print(f"  ❌ Failed to extract detection info")
        return False
    
    print(f"  📐 Rotation: {detection['angle_deg']:.1f}° (conf: {detection['confidence']:.2f})")
    
    # De-rotate and crop
    derotated = de_rotate_crop(img, detection)
    if derotated is None:
        print(f"  ❌ Failed to de-rotate")
        return False
    
    print(f"  📦 Cropped size: {derotated.shape[1]}x{derotated.shape[0]}")
    
    # Save debug image
    debug_path = image_path.replace('.png', '_derotated.png').replace('.jpg', '_derotated.jpg')
    cv2.imwrite(debug_path, derotated)
    print(f"  💾 Saved: {debug_path}")
    
    # Try to decode
    content = try_decode(derotated)
    if content:
        print(f"  ✅ DECODED: {content[:50]}...")
        return True
    else:
        print(f"  ⚠️ Detection OK, but decoding failed")
        return False

def main():
    print("="*60)
    print("OBB MODEL VERIFICATION")
    print("="*60)
    
    if not os.path.exists(MODEL_PATH):
        print(f"❌ Model not found: {MODEL_PATH}")
        return
    
    print(f"Loading model: {MODEL_PATH}")
    model = YOLO(MODEL_PATH)
    print(f"Model task: {model.task}")
    
    if model.task != 'obb':
        print("⚠️ WARNING: Model is not OBB type!")
    
    # Test specific challenging images
    print("\n" + "="*60)
    print("PHASE 1: Challenging Test Images")
    print("="*60)
    
    phase1_success = 0
    phase1_total = 0
    
    for img_path in TEST_IMAGES:
        if os.path.exists(img_path):
            phase1_total += 1
            if test_image(model, img_path):
                phase1_success += 1
    
    if phase1_total > 0:
        print(f"\nPhase 1 Results: {phase1_success}/{phase1_total} ({100*phase1_success/phase1_total:.1f}%)")
    
    # Test validation set (sample)
    print("\n" + "="*60)
    print("PHASE 2: Validation Set Sample (10 images)")
    print("="*60)
    
    val_images = glob.glob(f"{VAL_DIR}/*.jpg")[:10]
    
    phase2_success = 0
    phase2_detect = 0
    
    for img_path in val_images:
        results = model.predict(img_path, conf=CONF_THRESHOLD, verbose=False)
        result = results[0]
        
        if result.obb is not None and len(result.obb) > 0:
            phase2_detect += 1
            
            img = cv2.imread(img_path)
            detection = get_rotation_angle(result)
            if detection:
                derotated = de_rotate_crop(img, detection)
                if derotated is not None and try_decode(derotated):
                    phase2_success += 1
    
    print(f"\nPhase 2 Results:")
    print(f"  Detected: {phase2_detect}/{len(val_images)}")
    print(f"  Decoded:  {phase2_success}/{len(val_images)}")
    
    # Summary
    print("\n" + "="*60)
    print("SUMMARY")
    print("="*60)
    total_success = phase1_success + phase2_success
    total_tests = phase1_total + len(val_images)
    print(f"Total Decode Success: {total_success}/{total_tests}")
    
    if phase1_total > 0 and phase1_success == phase1_total:
        print("✅ All challenging images passed!")
    elif phase1_success > 0:
        print("⚠️ Partial success on challenging images")
    else:
        print("❌ Failed on challenging images")

if __name__ == "__main__":
    main()
