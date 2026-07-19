from huggingface_hub import HfApi

def check():
    api = HfApi()
    repo_id = "Piero2411/YOLOV8s-Barcode-Detection"
    print(f"Checking {repo_id}...")
    files = api.list_repo_files(repo_id)
    print(files)

if __name__ == "__main__":
    check()
