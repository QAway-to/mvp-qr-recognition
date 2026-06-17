from roboflow import Roboflow
from ultralytics import YOLO
import torch
import os
import shutil

# --- Configuration ---
API_KEY = "0TZqoy920xvUzqwzQtS6"
WORKSPACE = "mustafa-aktan"
PROJECT = "qr-code-ofk26"
VERSION = 2
MODEL_TYPE = "yolo11n.pt"  # Use nano model for speed/size
EPOCHS_GPU = 50
EPOCHS_CPU = 10
EXPORT_FORMAT = "onnx"

def main():
    print("=== Starting QR Code Model Training Pipeline ===")
    
    # 1. Dataset Preparation
    print("\n--- Step 1: Downloading Dataset ---")
    rf = Roboflow(api_key=API_KEY)
    project = rf.workspace(WORKSPACE).project(PROJECT)
    version = project.version(VERSION)
    dataset = version.download("yolov11")
    
    # Verify data.yaml path
    data_yaml_path = os.path.join(dataset.location, "data.yaml")
    if not os.path.exists(data_yaml_path):
        print(f"Error: data.yaml not found at {data_yaml_path}")
        # Try to find it recursively? usually it is in the root of dataset location
        return

    print(f"Dataset downloaded to: {dataset.location}")
    print(f"Data config: {data_yaml_path}")

    # 2. Environment & Training
    print("\n--- Step 2: Training Environment ---")
    
    # Check GPU
    device = 'cuda' if torch.cuda.is_available() else 'cpu'
    print(f"Using device: {device}")
    
    if device == 'cuda':
        print(f"GPU: {torch.cuda.get_device_name(0)}")
        epochs = EPOCHS_GPU
    else:
        print("WARNING: GPU not found. Training on CPU (slow). Reducing epochs.")
        epochs = EPOCHS_CPU

    print(f"Training {MODEL_TYPE} for {epochs} epochs...")
    
    # Load model
    model = YOLO(MODEL_TYPE) 
    
    # Train
    results = model.train(
        data=data_yaml_path,
        epochs=epochs,
        imgsz=640,
        device=0 if device == 'cuda' else 'cpu',
        project="qr_training_run",
        name="run1",
        exist_ok=True
    )
    
    print("\n--- Training Complete ---")
    best_model_path = os.path.join("qr_training_run", "run1", "weights", "best.pt")
    print(f"Best model saved at: {best_model_path}")

    # 3. Validation & Export
    print("\n--- Step 3: Validation & Export ---")
    
    # Validate
    metrics = model.val()
    print(f"mAP50-95: {metrics.box.map}")
    print(f"mAP50: {metrics.box.map50}")

    # Export to ONNX
    print(f"\nExporting to {EXPORT_FORMAT}...")
    onnx_path = model.export(format=EXPORT_FORMAT)
    print(f"Model exported to: {onnx_path}")
    
    # Copy ONNX to a convenient location
    final_output = "best.onnx"
    if os.path.exists(str(onnx_path)): # export returns path string
        shutil.copy(str(onnx_path), final_output)
        print(f"SUCCESS: Exported model ready at ./{final_output}")
    else:
         # Sometimes export returns filename, sometimes path. 
         # Ultralytics V8 export usually saves in same dir as weights or source file.
         # Let's check common locations if onnx_path is just a string name
         possible_path = os.path.join("qr_training_run", "run1", "weights", "best.onnx")
         if os.path.exists(possible_path):
             shutil.copy(possible_path, final_output)
             print(f"SUCCESS: Exported model ready at ./{final_output}")
         else:
             print(f"WARNING: Could not locate exported ONNX file automatically. Check logs. Returned: {onnx_path}")

    # 4. Inference on Test Images
    print("\n--- Step 4: Generating Example Results ---")
    test_images_dir = os.path.join(dataset.location, "test", "images")
    results_dir = "results"
    
    if os.path.exists(test_images_dir):
        os.makedirs(results_dir, exist_ok=True)
        # Predict on all images in test folder
        model.predict(source=test_images_dir, save=True, project="qr_training_run", name="inference", exist_ok=True, conf=0.5)
        print(f"Inference results saved to qr_training_run/inference")
    else:
        print("Test images directory not found, skipping inference preview.")

if __name__ == "__main__":
    main()
