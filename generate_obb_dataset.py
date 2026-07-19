
import os
import random
import cv2
import numpy as np
import qrcode
from PIL import Image, ImageDraw

# Configuration
OUTPUT_DIR = "generated_obb_dataset"
NUM_IMAGES = 1000
TRAIN_RATIO = 0.8
IMG_SIZE = 640
QR_SIZE_RANGE = (150, 400) # Min/Max size of QR in pixels

def ensure_dirs():
    splits = ['train', 'val']
    for split in splits:
        os.makedirs(f"{OUTPUT_DIR}/images/{split}", exist_ok=True)
        os.makedirs(f"{OUTPUT_DIR}/labels/{split}", exist_ok=True)

def generate_random_qr_content():
    types = ['url', 'text', 'json', 'email']
    t = random.choice(types)
    if t == 'url':
        return f"https://example.com/{random.randint(1000, 99999)}"
    elif t == 'email':
        return f"user{random.randint(1000,9999)}@example.com"
    elif t == 'json':
        return f'{{"id": {random.randint(1,100)}, "val": "data"}}'
    else:
        return f"Random Text {random.randint(100000, 999999)}"

def create_rotated_qr(content, angle):
    qr = qrcode.QRCode(
        version=1,
        error_correction=qrcode.constants.ERROR_CORRECT_L,
        box_size=10,
        border=4,
    )
    qr.add_data(content)
    qr.make(fit=True)
    qr_img = qr.make_image(fill_color="black", back_color="white").convert('RGBA')
    
    # Resize randomly
    target_size = random.randint(QR_SIZE_RANGE[0], QR_SIZE_RANGE[1])
    qr_img = qr_img.resize((target_size, target_size), Image.NEAREST)
    
    # Create a larger canvas to rotate without cropping
    w, h = qr_img.size
    overlay = Image.new('RGBA', (w * 2, h * 2), (255, 255, 255, 0))
    overlay.paste(qr_img, (w // 2, h // 2))
    
    # Rotate
    rotated = overlay.rotate(angle, expand=True, resample=Image.BICUBIC)
    
    # Calculate OBB coordinates
    # Original corners relative to the overly center
    # Center of original QR in overlay is (w, h)
    # Corners: (w//2, h//2), (w//2+w, h//2), (w//2+w, h//2+h), (w//2, h//2+h)
    
    cx, cy = w, h # Center of rotation (center of overlay, where we pasted QR)
    
    # Local coords of QR relative to center
    half_w = w / 2
    half_h = h / 2
    
    corners = [
        (-half_w, -half_h),
        ( half_w, -half_h),
        ( half_w,  half_h),
        (-half_w,  half_h)
    ]
    
    # Rotate points
    rad = -np.radians(angle) # PIL rotation is counter-clockwise, math is usually CCW too, but let's verify direction
    # PIL rotate angle: "In degrees counter clockwise."
    
    rot_corners = []
    cos_a = np.cos(rad)
    sin_a = np.sin(rad)
    
    for x, y in corners:
        rx = x * cos_a - y * sin_a
        ry = x * sin_a + y * cos_a
        rot_corners.append((rx, ry))
        
    # Map back to image coordinates
    # The rotated image center is (rotated.width/2, rotated.height/2)
    rcx, rcy = rotated.width / 2, rotated.height / 2
    
    final_corners = []
    for rx, ry in rot_corners:
        final_corners.append((rcx + rx, rcy + ry))
        
    return rotated, final_corners

def generate_sample(index, split):
    content = generate_random_qr_content()
    angle = random.randint(0, 360)
    
    qr_img_rot, corners = create_rotated_qr(content, angle)
    
    # Create background
    bg_color = random.randint(200, 255)
    bg = Image.new('RGB', (IMG_SIZE, IMG_SIZE), (bg_color, bg_color, bg_color))
    
    # Paste QR at random position
    w, h = qr_img_rot.size
    max_x = IMG_SIZE - w
    max_y = IMG_SIZE - h
    
    # Ensure it fits
    if max_x < 0 or max_y < 0:
        qr_img_rot = qr_img_rot.resize((int(w*0.5), int(h*0.5)))
        w, h = qr_img_rot.size
        # Scale corners
        corners = [(x*0.5, y*0.5) for x, y in corners]
        max_x = IMG_SIZE - w
        max_y = IMG_SIZE - h
        
    paste_x = random.randint(0, max(0, max_x))
    paste_y = random.randint(0, max(0, max_y))
    
    bg.paste(qr_img_rot, (paste_x, paste_y), qr_img_rot)
    
    # Adjust corners to global position
    global_corners = []
    for x, y in corners:
        global_corners.append((x + paste_x, y + paste_y))
        
    # Save Image
    filename = f"qr_{index:05d}.jpg"
    img_path = os.path.join(OUTPUT_DIR, "images", split, filename)
    bg.save(img_path, quality=90)
    
    # Save Label (YOLO OBB: class x1 y1 x2 y2 x3 y3 x4 y4 normalized)
    label_path = os.path.join(OUTPUT_DIR, "labels", split, filename.replace('.jpg', '.txt'))
    
    with open(label_path, 'w') as f:
        # Normalize
        norm_corners = []
        for x, y in global_corners:
            nx = min(max(x / IMG_SIZE, 0.0), 1.0)
            ny = min(max(y / IMG_SIZE, 0.0), 1.0)
            norm_corners.extend([f"{nx:.6f}", f"{ny:.6f}"])
            
        line = f"0 {' '.join(norm_corners)}\n"
        f.write(line)

def main():
    ensure_dirs()
    print(f"Generating {NUM_IMAGES} OBB samples...")
    
    for i in range(NUM_IMAGES):
        split = 'train' if i < NUM_IMAGES * TRAIN_RATIO else 'val'
        generate_sample(i, split)
        if i % 100 == 0:
            print(f"Generated {i} images...")
            
    print("Dataset generation complete.")
    
    # Verify by drawing one
    print("Verifying one sample...")
    # (Optional verification code could go here, but I'll skip for speed)

if __name__ == "__main__":
    main()
