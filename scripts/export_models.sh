#!/usr/bin/env bash
# Create .venv and export the ONNX photo models. Safe to re-run.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

py="${PYTHON:-python3.12}"
if ! command -v "$py" >/dev/null 2>&1; then
  py=python3
fi

if [[ ! -x .venv/bin/python ]]; then
  "$py" -m venv .venv
fi
# shellcheck disable=SC1091
source .venv/bin/activate
python -m pip install --upgrade pip
# Torch and torchvision must come from the same CPU index. A PyPI torchvision
# wheel does not register torchvision::nms inside the CPU torch build.
pip install torch torchvision --index-url https://download.pytorch.org/whl/cpu
pip install spandrel spandrel_extra_arches pillow safetensors onnx
# PyPI torchvision sorts above the +cpu local version, so a plain install
# leaves the mismatched wheel in place. Force the CPU build.
pip install --force-reinstall --no-deps torchvision==0.29.2+cpu --index-url https://download.pytorch.org/whl/cpu
python scripts/export_models.py
