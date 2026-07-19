from ultralytics import YOLO
import sys

MODEL_PATH = r'C:\Users\sadov\Downloads\types\best.pt'

try:
    print(f"Loading model from {MODEL_PATH}...")
    model = YOLO(MODEL_PATH)
    print(f"Model Task: {model.task}")
    print(f"Model Names: {model.names}")
    
    if model.task != 'obb':
        print("WARNING: This is NOT an OBB model!")
    else:
        print("CONFIRMED: This IS an OBB model.")
        
except Exception as e:
    print(f"Error loading model: {e}")
