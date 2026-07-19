from ultralytics import YOLO
from huggingface_hub import hf_hub_download
import os
import shutil

def main():
    print("=== Exporting YOLOv8s (Piero2411) to INT8 ONNX ===")

    # 1. Download Model
    print("\n[1] Downloading Model (Piero2411)...")
    try:
        model_path = hf_hub_download(repo_id="Piero2411/YOLOV8s-Barcode-Detection", filename="YOLOV8s_Barcode_Detection.pt")
        print(f"Downloaded: {model_path}")
    except Exception as e:
        print(f"Download failed: {e}")
        return

    # 2. Export INT8
    print("\n[2] Exporting to INT8 ONNX...")
    model = YOLO(model_path)
    
    # Needs 'data' arg for INT8 calibration
    # Assuming we run this from 'training/' dir, so data path should be relative or absolute
    data_yaml = os.path.abspath("yolo-2/data.yaml")
    
    try:
        # Export with int8=True
        # imgsz=640 is standard
        export_path = model.export(format="onnx", imgsz=640, int8=True, data=data_yaml)
        print(f"Exported to: {export_path}")
        
        # Move to public/model.onnx
        target_dir = os.path.abspath("../public")
        os.makedirs(target_dir, exist_ok=True)
        target_file = os.path.join(target_dir, "model.onnx")
        
        if os.path.exists(export_path):
            shutil.copy(export_path, target_file)
            print(f"SUCCESS: Model copied to {target_file}")
            print(f"New Size: {os.path.getsize(target_file) / 1024 / 1024:.2f} MB")
        else:
            print("Export file not found via return path, checking local dir...")
            # Fallback check
            possible_name = "YOLOV8s_Barcode_Detection.onnx"
            if os.path.exists(possible_name):
                 shutil.copy(possible_name, target_file)
                 print(f"SUCCESS: Model copied to {target_file}")
            
    except Exception as e:
        print(f"Export Error: {e}")

if __name__ == "__main__":
    main()
