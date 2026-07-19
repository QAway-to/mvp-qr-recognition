from huggingface_hub import HfApi, hf_hub_download

from huggingface_hub import HfApi

def deep_search():
    api = HfApi()
    
    # 1. Search by Author
    authors = ["keremberke", "DunnBC22", "bilalcevik", "foduucom", "mshamrai"]
    for author in authors:
        print(f"\nScanning author: {author}...")
        try:
            models = api.list_models(author=author, limit=1000, sort="downloads", direction=-1)
            for m in models:
                if "qr" in m.id.lower() or "bar" in m.id.lower():
                    print(f"!!! FOUND !!! {m.id} (Downloads: {m.downloads})")
        except Exception as e:
            print(f"Error scanning {author}: {e}")

    print("\nScanning 'yolov8' deep search...")
    # Search for just 'yolov8' to get maximal candidates, then filter locally
    models = api.list_models(search="yolov8", limit=2000, sort="downloads", direction=-1)
    
    count = 0
    for m in models:
         if "qr" in m.id.lower() or "bar" in m.id.lower():
             print(f"Candidate: {m.id} (Downloads: {m.downloads})")
             count += 1
    print(f"Total candidates found: {count}")

if __name__ == "__main__":
    deep_search()
