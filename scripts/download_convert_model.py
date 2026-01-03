import os
import shutil
from qrdet import QRDetector
from ultralytics import YOLO

def main():
    print("Starting model download and conversion...")
    
    # 1. Define a local temporary folder for weights so we know exactly where they are
    weights_dir = os.path.abspath("temp_weights")
    if not os.path.exists(weights_dir):
        os.makedirs(weights_dir)
        
    print(f"Downloading weights to: {weights_dir}")
    
    # 2. Initialize QRDet with 'n' (nano) model.
    # This triggers the automatic download of the .pt file to our custom folder.
    try:
        detector = QRDetector(model_size='n', weights_folder=weights_dir)
        print("QRDet initialized and weights downloaded.")
    except Exception as e:
        print(f"Error initializing QRDet: {e}")
        return

    # 3. Locate the downloaded .pt file
    # We expect 'qrdet-n.pt' or similar
    pt_file = None
    for f in os.listdir(weights_dir):
        if f.endswith(".pt"):
            pt_file = os.path.join(weights_dir, f)
            break
            
    if not pt_file:
        print("Could not find .pt file in weights directory.")
        return
        
    print(f"Found PyTorch model: {pt_file}")
    
    # 4. Export to ONNX using Ultralytics
    print("Exporting to ONNX...")
    try:
        model = YOLO(pt_file)
        # opset=12 is widely compatible (e.g. with onnxruntime-web)
        exported_path = model.export(format="onnx", opset=12)
        print(f"Export successful: {exported_path}")
        
        # 5. Move to public/model.onnx
        target_path = os.path.abspath("public/model.onnx")
        
        # The export usually creates the .onnx file in the same dir as the .pt file
        # exported_path might be that path string.
        
        if os.path.exists(exported_path):
            shutil.move(exported_path, target_path)
            print(f"Moved ONNX model to: {target_path}")
        else:
            print(f"Exported file not found at {exported_path}")
            
    except Exception as e:
        print(f"Error during export: {e}")

    # Cleanup (optional, keeping for now in case of debug needs)
    # shutil.rmtree(weights_dir)

if __name__ == "__main__":
    main()
