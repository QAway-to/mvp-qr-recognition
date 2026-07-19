from ultralytics import YOLO
import sys

print("Starting mini verify...")
try:
    model = YOLO('current_best.pt')
    print(f"Model task: {model.task}")
    
    # Try explicit predict
    print("Running predict...")
    results = model.predict('generated_dataset/text_rot_45.png', task='obb', verbose=False, conf=0.05)
    
    print(f"Results list length: {len(results)}")
    
    if len(results) > 0:
        r = results[0]
        if r.obb:
            print(f"OBB Detections found: {len(r.obb)}")
            for box in r.obb:
                print(f"Conf: {box.conf}")
                print(f"Angle: {box.xywhr[0][-1]}") # xywhr format: x,y,w,h,rotation
        else:
            print("No OBB objects detected.")
            
except Exception as e:
    print(f"ERROR: {e}")
    import traceback
    traceback.print_exc()
