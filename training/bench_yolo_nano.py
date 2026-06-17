from roboflow import Roboflow
from ultralytics import YOLO
from huggingface_hub import hf_hub_download
import os
import shutil
from collections import defaultdict

def main():
    print("=== Benchmarking YOLOv8n (Nano) on Yolo-6uedu v2 ===")

    # 1. Download Dataset
    print("\n[1] Downloading Dataset...")
    rf = Roboflow(api_key="0TZqoy920xvUzqwzQtS6")
    project = rf.workspace("school-hcpx6").project("yolo-6uedu")
    version = project.version(2)
    dataset = version.download("yolov11")
    
    test_images_dir = os.path.join(dataset.location, "test", "images")
    print(f"Test Images Directory: {test_images_dir}")

    # 2. Setup Model (Piero2411 Small - Fallback)
    print("\n[2] Setting up Model (Piero2411/YOLOV8s-Barcode-Detection)...")
    try:
        # Try direct download from HF
        model_path = hf_hub_download(repo_id="Piero2411/YOLOV8s-Barcode-Detection", filename="YOLOV8s_Barcode_Detection.pt")
        print(f"Downloaded Model to: {model_path}")
    except Exception as e:
        print(f"HF Download failed: {e}")
        return

    # Load Model & Export to ONNX (FP16)
    model = YOLO(model_path)
    print("Exporting to ONNX (FP16)...")
    try:
        onnx_path = model.export(format="onnx", imgsz=640, half=True)
        print(f"Exported ONNX to: {onnx_path}")
    except Exception as e:
        print(f"Export warning: {e}")

    # 3. Benchmark
    print("\n[3] Running Inference...")
    results_stats = defaultdict(lambda: {"total": 0, "detected": 0})
    
    image_files = [f for f in os.listdir(test_images_dir) if f.lower().endswith(('.jpg', '.jpeg', '.png'))]
    print(f"Total Test Images: {len(image_files)}")
    
    for img_file in image_files:
        img_path = os.path.join(test_images_dir, img_file)
        
        # Determine category based on filename
        category = "Clean"
        lower_name = img_file.lower()
        if "rot" in lower_name or "angle" in lower_name:
            category = "Rotation"
        elif "noise" in lower_name or "grain" in lower_name:
            category = "Noise"
        elif "exp" in lower_name or "contrast" in lower_name or "bright" in lower_name or "dark" in lower_name:
            category = "Exposure"
        elif "blur" in lower_name:
            category = "Blur"
            
        results_stats[category]["total"] += 1
        results_stats["ALL"]["total"] += 1
        
        # Inference
        results = model.predict(img_path, conf=0.5, verbose=False)
        
        # Check if any QR detected (Class 0 or 1 depending on model, usually 0 for this specific model)
        # keremberke model typically has class 0 = 'qr code'
        detected = False
        if len(results) > 0 and len(results[0].boxes) > 0:
            detected = True
            
        if detected:
            results_stats[category]["detected"] += 1
            results_stats["ALL"]["detected"] += 1

    # 4. Report
    print("\n=== BENCHMARK REPORT ===")
    print(f"{'Category':<15} | {'Images':<8} | {'Detected':<8} | {'Accuracy':<8}")
    print("-" * 45)
    
    for cat, stats in results_stats.items():
        acc = (stats["detected"] / stats["total"]) * 100 if stats["total"] > 0 else 0
        print(f"{cat:<15} | {stats['total']:<8} | {stats['detected']:<8} | {acc:.1f}%")

if __name__ == "__main__":
    main()
