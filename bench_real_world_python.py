
import cv2
import numpy as np
from pyzbar.pyzbar import decode
import os
import random
import glob
from ultralytics import YOLO

# Constants
DATASET_DIR = "yolo-2/test/images"
BATCH_SIZE = 50
NUM_BATCHES = 4
YOLO_MODEL_PATH = "training/qr_training_run/run1/weights/best.pt" # Trying custom model first
if not os.path.exists(YOLO_MODEL_PATH):
    YOLO_MODEL_PATH = "yolov8n.pt"

def rotate_image(image, angle):
    image_center = tuple(np.array(image.shape[1::-1]) / 2)
    rot_mat = cv2.getRotationMatrix2D(image_center, angle, 1.0)
    result = cv2.warpAffine(image, rot_mat, image.shape[1::-1], flags=cv2.INTER_LINEAR)
    return result

def try_decode(img, method="ZBar", detector=None):
    if method == "ZBar":
        decoded = decode(img)
        if decoded:
            return True
    elif method == "WeChatQR" and detector:
        try:
            res, points = detector.detectAndDecode(img)
            if res and len(res) > 0 and len(res[0]) > 0:
                return True
        except:
            pass
    return False

def crop_qr(image, model):
    results = model(image, verbose=False)
    crops = []
    for result in results:
        for box in result.boxes:
            x1, y1, x2, y2 = box.xyxy[0].cpu().numpy().astype(int)
            # Add padding
            h, w = image.shape[:2]
            p = 10
            x1 = max(0, x1 - p)
            y1 = max(0, y1 - p)
            x2 = min(w, x2 + p)
            y2 = min(h, y2 + p)
            
            crop = image[y1:y2, x1:x2]
            if crop.size > 0:
                crops.append(crop)
    return crops

def test_image(path, detector, yolo_model):
    img = cv2.imread(path)
    if img is None:
        return False
        
    # 0. YOLO Crop (Try generic detection first)
    crops = crop_qr(img, yolo_model)
    
    # If YOLO found nothing, use original image. If it found stuff, test crops.
    images_to_test = crops if len(crops) > 0 else [img]
    
    for test_img in images_to_test:
        gray = cv2.cvtColor(test_img, cv2.COLOR_BGR2GRAY)
        
        methods = ["ZBar", "WeChatQR"]
        angles = [0, 30, -30, 45, -45] 
        scales = [1, 2] 
        
        # Cascade of attempts
        
        # 1. Standard Pass
        for method in methods:
            if try_decode(gray, method, detector):
                return True
                
        # 2. Inversion
        inverted = cv2.bitwise_not(gray)
        for method in methods:
            if try_decode(inverted, method, detector):
                return True

        # 3. Rotations
        for angle in [30, -30, 45, -45]:
            rotated = rotate_image(gray, angle)
            for method in methods:
                if try_decode(rotated, method, detector):
                    return True
                    
        # 4. Rotations + Thresholding
        for angle in [0, 30, -30, 45, -45]:
            rotated = rotate_image(gray, angle)
            _, thresh = cv2.threshold(rotated, 128, 255, cv2.THRESH_BINARY)
            for method in methods:
                if try_decode(thresh, method, detector):
                    return True

        # 5. Upscaling (Last Resort)
        h, w = gray.shape[:2]
        if h < 1000 and w < 1000:
            upscaled = cv2.resize(gray, (w*2, h*2), interpolation=cv2.INTER_LINEAR)
            for method in methods:
                if try_decode(upscaled, method, detector):
                    return True
                    
            upscaled_inv = cv2.bitwise_not(upscaled)
            for method in methods:
                if try_decode(upscaled_inv, method, detector):
                    return True

    return False

def init_wechat():
    detect_proto = "wechat_models/detect.prototxt"
    detect_caffe = "wechat_models/detect.caffemodel"
    sr_proto = "wechat_models/sr.prototxt"
    sr_caffe = "wechat_models/sr.caffemodel"
    
    try:
        if os.path.exists(detect_proto) and os.path.exists(detect_caffe):
            return cv2.wechat_qrcode_WeChatQRCode(detect_proto, detect_caffe, sr_proto, sr_caffe)
    except AttributeError:
        print("Warning: WeChatQRCode not available in this OpenCV build. Using ZBar only.")
        return None
    return None

def main():
    print(f"Starting Benchmark on {DATASET_DIR}")
    print(f"Plan: {NUM_BATCHES} batches of {BATCH_SIZE} images each.")
    
    all_images = glob.glob(os.path.join(DATASET_DIR, "*.jpg")) + \
                 glob.glob(os.path.join(DATASET_DIR, "*.png")) + \
                 glob.glob(os.path.join(DATASET_DIR, "*.jpeg"))
                 
    if not all_images:
        print("No images found!")
        return
        
    print(f"Found {len(all_images)} total images.")
    detector = init_wechat()
    
    print(f"Loading YOLO model from: {YOLO_MODEL_PATH}")
    yolo = YOLO(YOLO_MODEL_PATH)
    
    total_processed = 0
    total_success = 0
    
    for i in range(NUM_BATCHES):
        batch = random.sample(all_images, BATCH_SIZE)
        batch_success = 0
        
        print(f"\n--- Batch {i+1}/{NUM_BATCHES} ---")
        for img_path in batch:
            if test_image(img_path, detector, yolo):
                batch_success += 1
                # print(".", end="", flush=True)
            else:
                # print("x", end="", flush=True)
                pass
        
        accuracy = (batch_success / BATCH_SIZE) * 100
        print(f"\nBatch {i+1} Result: {batch_success}/{BATCH_SIZE} ({accuracy:.1f}%)")
        
        total_processed += BATCH_SIZE
        total_success += batch_success

    overall_acc = (total_success / total_processed) * 100
    print(f"\n=== FINAL RESULTS ===")
    print(f"Total Processed: {total_processed}")
    print(f"Total Success: {total_success}")
    print(f"Overall Accuracy: {overall_acc:.2f}%")

if __name__ == "__main__":
    main()
