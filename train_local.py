import os
import yaml
import ultralytics
from ultralytics import YOLO
import generate_obb_dataset_v2 as generator

# ===== CONFIGURATION =====
DATASET_DIR = os.path.abspath("dataset_v3_local")
NUM_IMAGES = 5000
EPOCHS = 50
BATCH_SIZE = 16 # Reduce if Out Of Memory (OOM) error occurs

def main():
    # 1. GENERATE DATASET
    print(f"\n{'='*40}")
    print(f"🚀 PHASE 1: GENERATING DATASET ({NUM_IMAGES} images)")
    print(f"{'='*40}")
    
    # Configure generator
    generator.OUTPUT_DIR = DATASET_DIR
    generator.NUM_IMAGES = NUM_IMAGES
    # generator.DRIVE_DIR is not used in local mode logic we assume,
    # as OUTPUT_DIR overrides.
    
    # Run generation
    generator.main()
    
    # 2. CREATE CONFIG (data.yaml)
    print(f"\n{'='*40}")
    print(f"📄 PHASE 2: CREATING CONFIG")
    print(f"{'='*40}")
    
    yaml_content = {
        'path': DATASET_DIR,
        'train': 'images/train',
        'val': 'images/val',
        'test': 'images/val', 
        'names': {0: 'qr_code'}
    }
    
    yaml_path = os.path.join(DATASET_DIR, 'data.yaml')
    with open(yaml_path, 'w') as f:
        yaml.dump(yaml_content, f)
    
    print(f"✅ Config saved to: {yaml_path}")
    
    # 3. TRAIN YOLO MODEL
    print(f"\n{'='*40}")
    print(f"🏋️ PHASE 3: TRAINING YOLOv8 OBB")
    print(f"{'='*40}")
    
    # Load model (start freshly from pretrained nano weight to fix angle bias)
    model = YOLO('yolov8n-obb.pt')
    
    results = model.train(
        data=yaml_path,
        epochs=EPOCHS,
        imgsz=640,
        batch=BATCH_SIZE,
        project='runs/obb_training',
        name='hardcore_local_v1',
        exist_ok=True,
        patience=15,    # Stop if no improvement for 15 epochs
        save=True,      # Save checkpoints
        cos_lr=True,    # Cosine learning rate scheduler for better convergence
        plots=True,     # Save training plots
        device='cpu'        # Use GPU 0. Change to 'cpu' if no GPU.
    )
    
    print(f"\n{'='*40}")
    print(f"🎉 DONE! Best model saved at: runs/obb_training/hardcore_local_v1/weights/best.pt")
    print(f"{'='*40}")

if __name__ == '__main__':
    main()
