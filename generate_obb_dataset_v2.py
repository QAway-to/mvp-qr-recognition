import os
import random
import cv2
import numpy as np
import qrcode
from PIL import Image, ImageDraw, ImageFont, ImageFilter, ImageEnhance
from io import BytesIO
import yaml

# ===== CONFIGURATION =====
# Default to local directory if DRIVE_DIR is not set (for local testing)
DRIVE_DIR = "." 
OUTPUT_DIR = "generated_obb_dataset_v2"
NUM_IMAGES = 5000
TRAIN_RATIO = 0.8
IMG_SIZE = 640
QR_SIZE_RANGE = (60, 450)

AUG_PROBS = {
    'jpeg_artifacts': 0.7,
    'gaussian_blur': 0.4,
    'motion_blur': 0.3,
    'gaussian_noise': 0.5,
    'brightness': 0.6,
    'contrast': 0.7, # Increased probability
    'shadow': 0.4,
    'perspective': 0.6, # Increased probability
    'logo': 0.3,
    'retail_text': 0.5,
    'specular_highlight': 0.4,
    'partial_overlap': 0.3, # Added partial overlap probability
}

LOGO_TYPES = ['circle', 'square', 'text']

RETAIL_TEXTS = [
    'Scan Here',
    'Price: $9.99',
    'Limited Offer!',
    'Product ID: 12345',
    'Thank You!',
    '20% OFF',
    'Pay Now',
    'Customer Info',
    'Expires Soon',
    'Special Deal'
]

FONT_SIZES = [20, 25, 30, 35, 40]

PARTIAL_OVERLAP_FACTOR_RANGE = (0.1, 0.5) # Percentage of QR size that can overlap outside image

def ensure_dirs():
    for split in ['train', 'val']:
        os.makedirs(f'{OUTPUT_DIR}/images/{split}', exist_ok=True)
        os.makedirs(f'{OUTPUT_DIR}/labels/{split}', exist_ok=True)

def generate_random_qr_content():
    types = ['url', 'text', 'json', 'email', 'phone']
    t = random.choice(types)
    if t == 'url':
        return f'https://example.com/{random.randint(1000, 99999)}'
    elif t == 'email':
        return f'user{random.randint(1000,9999)}@mail.com'
    elif t == 'json':
        return f'{{"id": {random.randint(1,1000)}, "val": {random.randint(100,9999)}}}'
    elif t == 'phone':
        return f'+7{random.randint(9000000000, 9999999999)}'
    else:
        return f'Data-{random.randint(100000, 999999)}'

def create_random_logo(size):
    logo_img = Image.new('RGBA', (size, size), (255, 255, 255, 0)) # Transparent background
    draw = ImageDraw.Draw(logo_img)
    logo_type = random.choice(LOGO_TYPES)

    color = (random.randint(0, 200), random.randint(0, 200), random.randint(0, 200), random.randint(180, 255)) # Semi-transparent color

    if logo_type == 'circle':
        draw.ellipse([size * 0.1, size * 0.1, size * 0.9, size * 0.9], fill=color)
    elif logo_type == 'square':
        draw.rectangle([size * 0.1, size * 0.1, size * 0.9, size * 0.9], fill=color)
    elif logo_type == 'text':
        try:
            font_size = int(size * 0.6)
            try:
                font = ImageFont.truetype("arial.ttf", font_size)
            except IOError:
                font = ImageFont.load_default()
                # font_variant is not available in all PIL versions, rollback to simple load_default
                # font = ImageFont.load_default().font_variant(size=font_size) 
                pass

            text_content = random.choice('ABCDEFGHIJKLMNOPQRSTUVWXYZ123')
            # PIL < 10 handle textbbox differently or not at all, use simplistic fallback if needed
            if hasattr(draw, 'textbbox'):
                text_width, text_height = draw.textbbox((0,0), text_content, font=font)[2:]
            else:
                text_width, text_height = draw.textsize(text_content, font=font)
                
            text_x = (size - text_width) / 2
            text_y = (size - text_height) / 2
            draw.text((text_x, text_y), text_content, font=font, fill=color)
        except Exception as e:
            # Fallback to square if font issues
            draw.rectangle([size * 0.3, size * 0.3, size * 0.7, size * 0.7], fill=color)

    return logo_img

def create_qr_image():
    content = generate_random_qr_content()
    version = random.randint(1, 8)
    error_correction = random.choice([
        qrcode.constants.ERROR_CORRECT_L,
        qrcode.constants.ERROR_CORRECT_M,
        qrcode.constants.ERROR_CORRECT_Q,
        qrcode.constants.ERROR_CORRECT_H,
    ])
    box_size = random.randint(6, 14)
    border = random.randint(2, 6)

    if random.random() < 0.15:
        fill_color = (random.randint(0, 60), random.randint(0, 60), random.randint(0, 60))
        back_color = (random.randint(200, 255), random.randint(200, 255), random.randint(200, 255))
    else:
        fill_color = 'black'
        back_color = 'white'

    qr = qrcode.QRCode(version=version, error_correction=error_correction, box_size=box_size, border=border)
    qr.add_data(content)
    qr.make(fit=True)
    qr_img = qr.make_image(fill_color=fill_color, back_color=back_color).convert('RGBA')

    # Overlay logo if probability allows
    if random.random() < AUG_PROBS['logo']:
        qr_width, qr_height = qr_img.size
        logo_size = int(min(qr_width, qr_height) * random.uniform(0.2, 0.3))
        logo = create_random_logo(logo_size)
        logo_x = (qr_width - logo_size) // 2
        logo_y = (qr_height - logo_size) // 2
        qr_img.paste(logo, (logo_x, logo_y), logo)

    target_size = random.randint(QR_SIZE_RANGE[0], QR_SIZE_RANGE[1])
    qr_img = qr_img.resize((target_size, target_size), Image.LANCZOS)
    return qr_img

def rotate_qr(qr_img, angle):
    # Rotate the image with expand=True to fit the rotated content
    # We don't need the 2x canvas if we just want the content
    rotated = qr_img.rotate(angle, expand=True, resample=Image.BICUBIC)
    
    # Calculate corners
    w, h = qr_img.size
    # Original corners centered at (0,0)
    # Note: in PIL rotate, the center of rotation is the center of the image
    half_w, half_h = w / 2.0, h / 2.0
    corners = [(-half_w, -half_h), (half_w, -half_h), (half_w, half_h), (-half_w, half_h)]
    
    rad = -np.radians(angle) # PIL rotates counter-clockwise? standard is CCW.
    # Check PIL rotation direction. PIL rotate(angle) rotates counter-clockwise.
    # Standard math rotation matrix for CCW:
    # [ cos -sin ]
    # [ sin  cos ]
    # But for image coords (y down), a CCW rotation usually requires:
    # x' = x cos - y sin
    # y' = x sin + y cos
    # Let's stick to the previous successfully verified logic, just adapted for the center shift.
    
    cos_a, sin_a = np.cos(rad), np.sin(rad)
    rot_corners = [(x * cos_a - y * sin_a, x * sin_a + y * cos_a) for x, y in corners]
    
    # After expand=True, the new center is at (rotated.width/2, rotated.height/2)
    rcx, rcy = rotated.size[0] / 2.0, rotated.size[1] / 2.0
    final_corners = [(rcx + rx, rcy + ry) for rx, ry in rot_corners]
    
    return rotated, final_corners

def create_background():
    bg_type = random.choice(['solid', 'gradient', 'noise'])
    if bg_type == 'solid':
        color = random.randint(180, 255)
        return Image.new('RGB', (IMG_SIZE, IMG_SIZE), (color, color, color))
    elif bg_type == 'gradient':
        arr = np.zeros((IMG_SIZE, IMG_SIZE, 3), dtype=np.uint8)
        c1 = np.array([random.randint(150, 255) for _ in range(3)])
        c2 = np.array([random.randint(150, 255) for _ in range(3)])
        for y in range(IMG_SIZE):
            t = y / IMG_SIZE
            arr[y, :] = (c1 * (1 - t) + c2 * t).astype(np.uint8)
        return Image.fromarray(arr)
    else:
        noise = np.random.randint(180, 255, (IMG_SIZE, IMG_SIZE, 3), dtype=np.uint8)
        return Image.fromarray(noise)

def add_retail_text(img_pil):
    if random.random() < AUG_PROBS['retail_text']:
        draw = ImageDraw.Draw(img_pil)
        text_content = random.choice(RETAIL_TEXTS)
        font_size = random.choice(FONT_SIZES)
        text_color = (random.randint(0, 100), random.randint(0, 100), random.randint(0, 100)) # Dark colors

        try:
            try:
                font = ImageFont.truetype("arial.ttf", font_size)
            except IOError:
                font = ImageFont.load_default()
                # font_variant unavailable in some PIL versions
                pass
        except Exception as e:
            # print(f"Warning: Could not load font for retail text: {e}. Skipping text.")
            return img_pil

        if hasattr(draw, 'textbbox'):
            bbox = draw.textbbox((0,0), text_content, font=font)
            text_width = bbox[2] - bbox[0]
            text_height = bbox[3] - bbox[1]
        else:
             text_width, text_height = draw.textsize(text_content, font=font)

        placement_attempts = 0
        placed = False
        while not placed and placement_attempts < 10:
            x = random.randint(0, IMG_SIZE - text_width)
            y = random.randint(0, IMG_SIZE - text_height)

            center_x_start = IMG_SIZE * 0.25
            center_x_end = IMG_SIZE * 0.75
            center_y_start = IMG_SIZE * 0.25
            center_y_end = IMG_SIZE * 0.75

            # Avoid placing text exactly in the center where QR usually is (unless overlap allowed, but we try to avoid direct cover)
            if not (center_x_start < x < center_x_end and center_y_start < y < center_y_end) and \
               not (center_x_start < x + text_width < center_x_end and center_y_start < y + text_height < center_y_end):
                draw.text((x, y), text_content, font=font, fill=text_color)
                placed = True
            placement_attempts += 1

    return img_pil

def apply_jpeg_artifacts(img):
    quality = random.randint(15, 60)
    buffer = BytesIO()
    img.save(buffer, format='JPEG', quality=quality)
    buffer.seek(0)
    return Image.open(buffer).convert('RGB')

def apply_gaussian_blur(img):
    return img.filter(ImageFilter.GaussianBlur(radius=random.uniform(0.5, 3.0)))

def apply_motion_blur(img):
    arr = np.array(img)
    size = random.choice([5, 7, 9, 11])
    kernel = np.zeros((size, size))
    kernel[size // 2, :] = 1 / size
    angle = random.randint(0, 180)
    M = cv2.getRotationMatrix2D((size / 2, size / 2), angle, 1)
    kernel = cv2.warpAffine(kernel, M, (size, size))
    kernel = kernel / (kernel.sum() + 1e-6)
    blurred = cv2.filter2D(arr, -1, kernel)
    return Image.fromarray(blurred)

def apply_gaussian_noise(img):
    arr = np.array(img).astype(np.float32)
    noise = np.random.normal(0, random.uniform(5, 30), arr.shape)
    return Image.fromarray(np.clip(arr + noise, 0, 255).astype(np.uint8))

def apply_brightness(img):
    return ImageEnhance.Brightness(img).enhance(random.uniform(0.4, 1.6))

def apply_contrast(img):
    # Expanded contrast range
    return ImageEnhance.Contrast(img).enhance(random.uniform(0.3, 1.8))

def apply_shadow(img):
    arr = np.array(img).astype(np.float32)
    h, w = arr.shape[:2]
    x1, y1 = random.randint(0, w//2), random.randint(0, h//2)
    x2, y2 = random.randint(w//2, w), random.randint(h//2, h)
    arr[y1:y2, x1:x2] *= random.uniform(0.3, 0.7)
    return Image.fromarray(np.clip(arr, 0, 255).astype(np.uint8))

def apply_perspective(img, corners):
    arr = np.array(img)
    h, w = arr.shape[:2]
    # Expanded perspective strength range
    strength = random.uniform(0.08, 0.30)
    src_pts = np.float32([[0, 0], [w, 0], [w, h], [0, h]])
    dst_pts = src_pts.copy()
    for i in range(4):
        dst_pts[i, 0] += random.uniform(-strength * w, strength * w)
        dst_pts[i, 1] += random.uniform(-strength * h, strength * h)
    M = cv2.getPerspectiveTransform(src_pts, dst_pts)
    warped = cv2.warpPerspective(arr, M, (w, h), borderValue=(200, 200, 200))
    new_corners = []
    for cx, cy in corners:
        pt = np.array([[[cx, cy]]], dtype=np.float32)
        transformed = cv2.perspectiveTransform(pt, M)
        new_corners.append((transformed[0, 0, 0], transformed[0, 0, 1]))
    return Image.fromarray(warped), new_corners

def apply_specular_highlight(img):
    arr = np.array(img).astype(np.float32)
    h, w = arr.shape[:2]

    # Randomly place the highlight center and size
    center_x, center_y = random.randint(w // 4, 3 * w // 4), random.randint(h // 4, 3 * h // 4)
    radius = random.randint(min(h, w) // 4, min(h, w) // 2)
    intensity = random.uniform(0.3, 0.7) # How much brighter the highlight is
    feather = random.randint(20, 80) # How soft the edges are

    # Create a gradient mask for the highlight
    Y, X = np.ogrid[:h, :w]
    dist_from_center = np.sqrt((X - center_x)**2 + (Y - center_y)**2)

    mask = 1 - np.clip(dist_from_center / radius, 0, 1)
    mask = np.power(mask, random.uniform(0.5, 2.0)) # Control falloff curve
    # Ensure kernel size is odd
    feather = feather if feather % 2 == 1 else feather + 1
    mask = cv2.GaussianBlur(mask, (0,0), feather) # Feather the edges
    mask = np.expand_dims(mask, axis=-1)

    # Apply highlight: brighten the image in the mask area
    highlighted_arr = arr + (arr * mask * intensity)
    return Image.fromarray(np.clip(highlighted_arr, 0, 255).astype(np.uint8))

def apply_augmentations(img, corners):
    if random.random() < AUG_PROBS['perspective']:
        img, corners = apply_perspective(img, corners)
    if random.random() < AUG_PROBS['gaussian_blur']:
        img = apply_gaussian_blur(img)
    if random.random() < AUG_PROBS['motion_blur']:
        img = apply_motion_blur(img)
    if random.random() < AUG_PROBS['gaussian_noise']:
        img = apply_gaussian_noise(img)
    if random.random() < AUG_PROBS['brightness']:
        img = apply_brightness(img)
    if random.random() < AUG_PROBS['contrast']:
        img = apply_contrast(img)
    if random.random() < AUG_PROBS['shadow']:
        img = apply_shadow(img)
    if random.random() < AUG_PROBS['jpeg_artifacts']:
        img = apply_jpeg_artifacts(img)
    if random.random() < AUG_PROBS['specular_highlight']:
        img = apply_specular_highlight(img)
    return img, corners

def generate_sample(index, split):
    bg = create_background()
    bg = add_retail_text(bg)

    qr_img = create_qr_image()
    angle = random.uniform(0, 360)
    qr_rotated, corners = rotate_qr(qr_img, angle)

    qr_w, qr_h = qr_rotated.size

    # Modify paste_x and paste_y calculation to allow partial overlap
    if random.random() < AUG_PROBS['partial_overlap']:
        overlap_factor = random.uniform(PARTIAL_OVERLAP_FACTOR_RANGE[0], PARTIAL_OVERLAP_FACTOR_RANGE[1])
        # Allow QR code to start up to `overlap_factor` percentage outside the left/top edge
        min_paste_x = -int(qr_w * overlap_factor)
        min_paste_y = -int(qr_h * overlap_factor)
        # Allow QR code to end up to `overlap_factor` percentage outside the right/bottom edge
        max_paste_x = IMG_SIZE - int(qr_w * (1 - overlap_factor))
        max_paste_y = IMG_SIZE - int(qr_h * (1 - overlap_factor))

        paste_x = random.randint(min_paste_x, max_paste_x)
        paste_y = random.randint(min_paste_y, max_paste_y)
    else:
        # Original logic: ensure QR is fully within frame
        max_x, max_y = IMG_SIZE - qr_w, IMG_SIZE - qr_h
        if max_x < 0 or max_y < 0: # If QR is larger than image, scale down
            scale = min(IMG_SIZE / qr_w, IMG_SIZE / qr_h) * 0.9
            new_size = (int(qr_w * scale), int(qr_h * scale))
            qr_rotated = qr_rotated.resize(new_size, Image.LANCZOS)
            corners = [(x * scale, y * scale) for x, y in corners]
            qr_w, qr_h = qr_rotated.size
            max_x, max_y = IMG_SIZE - qr_w, IMG_SIZE - qr_h
        paste_x = random.randint(0, max(0, max_x))
        paste_y = random.randint(0, max(0, max_y))

    bg.paste(qr_rotated, (paste_x, paste_y), qr_rotated)
    global_corners = [(x + paste_x, y + paste_y) for x, y in corners]
    bg, global_corners = apply_augmentations(bg, global_corners)

    filename = f'qr_{index:05d}.jpg'
    img_save_path = f'{OUTPUT_DIR}/images/{split}/{filename}'
    bg.save(img_save_path, quality=90)

    # YOLO OBB format: class x1 y1 x2 y2 x3 y3 x4 y4 (normalized)
    with open(f'{OUTPUT_DIR}/labels/{split}/{filename.replace(".jpg", ".txt")}', 'w') as f:
        norm = []
        for x, y in global_corners:
            # We must Clamp the coordinates for the LABEL, but in an intelligent way.
            # Actually, YOLO OBB supports coordinates slightly outside [0,1] but it's risky for some loss functions.
            # Standard practice: clamp to [0, 1].
            # Note: We must clamp CAREFULLY. If we clamp all corners to the edge, we lose the orientation info.
            # But the content of the image only exists within 0..1. The user's code suggests:
            norm.extend([f'{min(max(x/IMG_SIZE,0),1):.6f}', f'{min(max(y/IMG_SIZE,0),1):.6f}'])
        f.write(f"0 {' '.join(norm)}\n")

def main():
    ensure_dirs()
    print(f'Starting generation of {NUM_IMAGES} images...')
    for i in range(NUM_IMAGES):
        split = 'train' if i < NUM_IMAGES * TRAIN_RATIO else 'val'
        generate_sample(i, split)
        if (i+1) % 100 == 0:
            print(f'Generated {i+1}/{NUM_IMAGES}')
    print('Done.')

if __name__ == "__main__":
    main()
