from ultralytics import YOLO
from huggingface_hub import hf_hub_download
import shutil
import os

def run():
    print("Downloading Piero2411/YOLOV8s-Barcode-Detection...")
    model_path = hf_hub_download(repo_id="Piero2411/YOLOV8s-Barcode-Detection", filename="YOLOV8s_Barcode_Detection.pt")
    
    print(f"Model downloaded to {model_path}")
    
    # Load model
    model = YOLO(model_path)
    
    # Export to ONNX (FP32 standard)
    # opset=12 is usually safe for tract
    # Using 416x416 for better speed (vs 640x640)
    print("Exporting to ONNX (FP32, 416px)...")
    path = model.export(format="onnx", opset=12, imgsz=416)
    
    print(f"Exported to {path}")
    
    # Move to public/model.onnx
    shutil.move(path, "public/model.onnx")
    print("Success: Moved to public/model.onnx")

if __name__ == "__main__":
    run()
