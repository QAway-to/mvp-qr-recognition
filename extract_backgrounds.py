"""
Background Extractor
Extracts images WITHOUT visible QR codes from the dataset
to use as realistic backgrounds for synthetic QR generation.
"""

import os
import cv2
import glob
from pyzbar.pyzbar import decode
from tqdm import tqdm
import shutil

# Configuration
SOURCE_DIRS = [
    "yolo-2/train/images",
    "yolo-2/test/images",
]
OUTPUT_DIR = "background_pool"
MAX_BACKGROUNDS = 2000  # Limit to avoid huge pool

def has_qr_code(img_path):
    """Check if image contains a detectable QR code."""
    try:
        img = cv2.imread(img_path)
        if img is None:
            return True  # Skip unreadable images
        
        gray = cv2.cvtColor(img, cv2.COLOR_BGR2GRAY)
        
        # Try ZBar detection
        decoded = decode(gray)
        if decoded:
            return True
        
        # Try inverted
        inverted = cv2.bitwise_not(gray)
        decoded = decode(inverted)
        if decoded:
            return True
        
        return False
    except Exception as e:
        print(f"Error processing {img_path}: {e}")
        return True  # Skip on error

def main():
    os.makedirs(OUTPUT_DIR, exist_ok=True)
    
    # Collect all source images
    all_images = []
    for src_dir in SOURCE_DIRS:
        all_images.extend(glob.glob(f"{src_dir}/*.jpg"))
        all_images.extend(glob.glob(f"{src_dir}/*.png"))
        all_images.extend(glob.glob(f"{src_dir}/*.jpeg"))
    
    print(f"Found {len(all_images)} total source images.")
    print(f"Scanning for QR-free backgrounds...")
    
    extracted = 0
    
    for img_path in tqdm(all_images, desc="Scanning"):
        if extracted >= MAX_BACKGROUNDS:
            break
        
        if not has_qr_code(img_path):
            # Copy to background pool
            filename = os.path.basename(img_path)
            dst_path = os.path.join(OUTPUT_DIR, f"bg_{extracted:05d}_{filename}")
            shutil.copy(img_path, dst_path)
            extracted += 1
    
    print(f"\nExtracted {extracted} QR-free backgrounds to {OUTPUT_DIR}/")
    
    if extracted < 100:
        print("\nWARNING: Very few backgrounds extracted!")
        print("This might be because most images contain QR codes.")
        print("Consider adding more diverse source images.")

if __name__ == "__main__":
    main()
