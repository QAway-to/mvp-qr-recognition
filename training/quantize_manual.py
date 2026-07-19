import onnx
from onnxruntime.quantization import quantize_dynamic, QuantType
import os
import shutil

def main():
    print("=== Post-Training Quantization (INT8) ===")
    
    # 1. We assume FP16 or FP32 model exists from previous step, or we re-export it as FP32 first
    # It's better to quantize from FP32.
    from ultralytics import YOLO
    from huggingface_hub import hf_hub_download
    
    print("Downloading Model...")
    model_path = hf_hub_download(repo_id="Piero2411/YOLOV8s-Barcode-Detection", filename="YOLOV8s_Barcode_Detection.pt")
    
    model = YOLO(model_path)
    # Export as standard FP32 first
    print("Exporting to standard ONNX (FP32)...")
    onnx_fp32_path = model.export(format="onnx", imgsz=640, half=False) 
    print(f"FP32 path: {onnx_fp32_path}")
    
    # 2. Quantize using ORT
    print("Quantizing to INT8 (Dynamic)...")
    output_model_path = "model_quantized.onnx"
    
    quantize_dynamic(
        model_input=onnx_fp32_path,
        model_output=output_model_path,
        weight_type=QuantType.QUInt8
    )
    
    print(f"Quantized Model Size: {os.path.getsize(output_model_path) / 1024 / 1024:.2f} MB")
    
    # 3. Move to public
    target_dir = os.path.abspath("../public")
    target_file = os.path.join(target_dir, "model.onnx")
    shutil.copy(output_model_path, target_file)
    print(f"Deployed to: {target_file}")

if __name__ == "__main__":
    main()
