
import urllib.request
import os

MODELS_DIR = "wechat_models"
if not os.path.exists(MODELS_DIR):
    os.makedirs(MODELS_DIR)

BASE_URL = "https://raw.githubusercontent.com/WeChatCV/opencv_3rdparty/wechat_qrcode/"
FILES = [
    "detect.prototxt",
    "detect.caffemodel",
    "sr.prototxt",
    "sr.caffemodel"
]

def download_models():
    for file in FILES:
        url = BASE_URL + file
        output = os.path.join(MODELS_DIR, file)
        if not os.path.exists(output):
            print(f"Downloading {file}...")
            try:
                urllib.request.urlretrieve(url, output)
                print(f"Downloaded {file}")
            except Exception as e:
                print(f"Failed to download {file}: {e}")
        else:
            print(f"{file} exists.")

if __name__ == "__main__":
    download_models()
