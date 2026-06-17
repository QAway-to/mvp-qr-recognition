from ultralytics import YOLO
import os
import shutil

def main():
    print("=== Deploying SOTA QR Model (keremberke/yolov8n-qr-code-detection) ===")
    
    # 1. Load Pre-trained Model from HF (Manual Download)
    from huggingface_hub import hf_hub_download
    print("Downloading model manually from HF (Piero2411/YOLOV8s-Barcode-Detection)...")
    model_path = hf_hub_download(repo_id="Piero2411/YOLOV8s-Barcode-Detection", filename="YOLOV8s_Barcode_Detection.pt")
    print(f"Downloaded to: {model_path}")

    model = YOLO(model_path)
    print(f"Model Classes: {model.names}")
    
    # 2. Export to ONNX
    print("Exporting to ONNX (imgsz=640)...")
    path = model.export(format="onnx", imgsz=640)
    print(f"Exported to: {path}")
    
    # 3. Move to public/model.onnx
    public_dir = os.path.join(os.getcwd(), "..", "public")
    target_path = os.path.join(public_dir, "model.onnx")
    
    os.makedirs(public_dir, exist_ok=True)
    
    # Export returns filename or path string
    if os.path.exists(str(path)):
        shutil.move(str(path), target_path)
        print(f"SUCCESS: Model deployed to {target_path}")
    else:
        # Fallback search
        possible_name = "yolov8n_qr_code_detection.onnx" # default name logic?
        if os.path.exists(possible_name):
             shutil.move(possible_name, target_path)
             print(f"SUCCESS: Model deployed to {target_path}")
        else:
             print(f"ERROR: Could not find exported file. Returned path: {path}")

if __name__ == "__main__":
    main()
