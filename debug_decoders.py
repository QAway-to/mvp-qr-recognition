
import cv2
import numpy as np
from pyzbar.pyzbar import decode
import os
import sys

# List of images that were failing in Rust
FAILING_IMAGES = [
    "generated_dataset/text_rot_45.png",
    "generated_dataset/text_rot_30.png",
    "generated_dataset/url_rot_45.png",
    "generated_dataset/url_rot_30.png",
    "generated_dataset/json_rot_45.png",
    "generated_dataset/json_rot_30.png",
    "generated_dataset/payment_rot_45.png",
    "generated_dataset/payment_rot_30.png"
]

def rotate_image(image, angle):
    image_center = tuple(np.array(image.shape[1::-1]) / 2)
    rot_mat = cv2.getRotationMatrix2D(image_center, angle, 1.0)
    result = cv2.warpAffine(image, rot_mat, image.shape[1::-1], flags=cv2.INTER_LINEAR)
    return result

def try_decode(img, name, method="ZBar"):
    if method == "ZBar":
        decoded = decode(img)
        if decoded:
            return True, decoded[0].data.decode("utf-8")
    elif method == "WeChatQR":
        detect_proto = "wechat_models/detect.prototxt"
        detect_caffe = "wechat_models/detect.caffemodel"
        sr_proto = "wechat_models/sr.prototxt"
        sr_caffe = "wechat_models/sr.caffemodel"
        
        if not os.path.exists(detect_proto):
            return False, "Models not found"
            
        try:
            detector = cv2.wechat_qrcode_WeChatQRCode(detect_proto, detect_caffe, sr_proto, sr_caffe)
            res, points = detector.detectAndDecode(img)
            if res and len(res) > 0 and len(res[0]) > 0:
                return True, res[0]
        except Exception as e:
            # print(f"WeChatQR Error: {e}")
            pass
    return False, None

def process_file(path):
    print(f"\n--- Testing: {path} ---")
    if not os.path.exists(path):
        print("File not found!")
        return

    img = cv2.imread(path)
    gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)

    methods = ["ZBar", "WeChatQR"]
    
    # 1. Original
    for method in methods:
        success, data = try_decode(gray, "Original", method)
        if success:
            print(f"[OK] SUCCESS (Original - {method}): {data[:20]}...")
            return

    print("[FAIL] Failed on Original. Trying Rotations...")

    # 2. Rotations
    angles = [30, -30, 45, -45]
    for method in methods:
        for angle in angles:
            rotated = rotate_image(gray, angle)
            success, data = try_decode(rotated, f"Rotated {angle}", method)
            if success:
                print(f"[OK] SUCCESS (Rotated {angle} deg - {method}): {data[:20]}...")
                return
            else:
                 print(f"   Failed (Rotated {angle} deg - {method})")

    # 3. Try Thresholding on Rotated
    print("[FAIL] Failed simple rotations. Trying Thresholding...")
    for method in methods:
        for angle in angles:
            rotated = rotate_image(gray, angle)
            _, thresh = cv2.threshold(rotated, 128, 255, cv2.THRESH_BINARY)
            success, data = try_decode(thresh, f"Rotated {angle} + Thresh", method)
            if success:
                print(f"[OK] SUCCESS (Rotated {angle} deg + Thresh - {method}): {data[:20]}...")
                return

    # 4. Try Inversion on Rotated (Common for payment QRs on dark backgrounds)
    print("[FAIL] Failed thresholding. Trying Inversion...")
    for method in methods:
        for angle in angles:
            rotated = rotate_image(gray, angle)
            inverted = cv2.bitwise_not(rotated)
            success, data = try_decode(inverted, f"Rotated {angle} + Invert", method)
            if success:
                print(f"[OK] SUCCESS (Rotated {angle} deg + Invert - {method}): {data[:20]}...")
                return

    # 4. Try Upscaling + Inversion (Last Resort for High Density)
    print("[FAIL] Failed standard. Trying Upscale (2x, 3x) + Inversion...")
    scales = [2, 3]
    for method in methods:
        for scale in scales:
            for angle in angles:
                 rotated = rotate_image(gray, angle)
                 # Upscale
                 h, w = rotated.shape[:2]
                 upscaled = cv2.resize(rotated, (w*scale, h*scale), interpolation=cv2.INTER_CUBIC)
                 
                 # Try Normal
                 success, data = try_decode(upscaled, f"Rotated {angle} + Scale {scale}x", method)
                 if success:
                     print(f"[OK] SUCCESS (Rotated {angle} deg + Scale {scale}x - {method}): {data[:20]}...")
                     return
                     
                 # Try Inverted
                 inverted = cv2.bitwise_not(upscaled)
                 success, data = try_decode(inverted, f"Rotated {angle} + Scale {scale}x + Invert", method)
                 if success:
                     print(f"[OK] SUCCESS (Rotated {angle} deg + Scale {scale}x + Invert - {method}): {data[:20]}...")
                     return

    print("[FAIL] FAILURE: Could not decode image with any method.")

def main():
    print("Starting Decoder Comparison (ZBar)...")
    for img_path in FAILING_IMAGES:
        process_file(img_path)

if __name__ == "__main__":
    main()
