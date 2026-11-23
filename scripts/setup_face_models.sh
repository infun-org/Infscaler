#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p models
fetch() {
  local url="$1" dest="$2"
  if [[ -s "$dest" ]]; then echo "have $dest"; return; fi
  local tmp="$dest.partial"
  curl -fL --retry 3 --retry-delay 2 "$url" -o "$tmp"
  mv "$tmp" "$dest"
}
# YuNet 2023mar, MIT, OpenCV Zoo. Static graph, standard YuNet output heads.
fetch "https://github.com/opencv/opencv_zoo/raw/main/models/face_detection_yunet/face_detection_yunet_2023mar.onnx" models/face_detection_yunet.onnx
# RestoreFormer++ FP16 ONNX conversion; upstream RestoreFormer++ code/model is Apache-2.0.
# Inference uses one aligned 512x512 crop at a time.
fetch "https://github.com/visomaster/visomaster-assets/releases/download/v0.1.0/RestoreFormerPlusPlus.fp16.onnx" models/RestoreFormerPlusPlus.onnx
echo '8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4  models/face_detection_yunet.onnx' | sha256sum -c -
echo 'e5df99ed4f501be2009ed8e708f407dd26ac400c55a43a01d8c8c157bc475b3f  models/RestoreFormerPlusPlus.onnx' | sha256sum -c -
echo "Face models installed. Provenance and licenses: models/MODELS.md"
