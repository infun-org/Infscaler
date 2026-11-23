#!/usr/bin/env bash
# Download the NCNN-Vulkan upscaler and the photo models InfScaler ships.
# Safe to re-run: files that are already present and large enough are kept.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/bin"
MODELS="$ROOT/models"
SAMPLES="$ROOT/samples"
mkdir -p "$BIN" "$MODELS" "$SAMPLES" "$SAMPLES/out"

UPSCAYL_URL="https://github.com/upscayl/upscayl-ncnn/releases/download/20251207-174704/upscayl-bin-20251207-174704-linux.zip"
REALESRGAN_URL="https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip"
NOMOS_PARAM_URL="https://github.com/upscayl/custom-models/raw/main/models/4xNomos8kSC.param"
NOMOS_BIN_URL="https://github.com/upscayl/custom-models/raw/main/models/4xNomos8kSC.bin"
ULTRA_PARAM_URL="https://github.com/upscayl/upscayl/raw/main/resources/models/ultrasharp-4x.param"
ULTRA_BIN_URL="https://github.com/upscayl/upscayl/raw/main/resources/models/ultrasharp-4x.bin"
BABOON_URL="https://github.com/upscayl/upscayl/raw/main/scripts/baboon.png"

UA="InfScalerSetup/0.1 (+https://github.com/upscayl/upscayl-ncnn)"

log() { printf '==> %s\n' "$*"; }

download() {
  local url="$1" dest="$2" min="$3"
  if [[ -f "$dest" ]] && [[ "$(stat -c%s "$dest")" -ge "$min" ]]; then
    log "already have $(basename "$dest")"
    return 0
  fi
  log "downloading $(basename "$dest")"
  curl -fL --retry 5 --retry-delay 2 -A "$UA" -o "$dest.partial" "$url"
  mv "$dest.partial" "$dest"
  local size
  size="$(stat -c%s "$dest")"
  if [[ "$size" -lt "$min" ]]; then
    echo "download too small ($size bytes): $dest" >&2
    exit 1
  fi
}

file_ok() {
  local path="$1" min="$2"
  [[ -f "$path" ]] && [[ "$(stat -c%s "$path")" -ge "$min" ]]
}

copy_matching() {
  local root="$1" name="$2" dest="$3"
  local found
  found="$(find "$root" -type f -name "$name" | head -n 1)"
  if [[ -z "$found" ]]; then
    echo "could not find $name inside the archive" >&2
    exit 1
  fi
  cp -f "$found" "$dest"
  local dir
  dir="$(dirname "$found")"
  find "$dir" -maxdepth 1 -type f -name '*.so*' -exec cp -f {} "$BIN/" \;
}

check_elf() {
  local path="$1"
  if ! file "$path" | grep -q 'ELF'; then
    echo "not an ELF binary: $path" >&2
    file "$path" >&2
    exit 1
  fi
  if ldd "$path" 2>&1 | grep -q 'not found'; then
    echo "missing shared libraries for $path" >&2
    ldd "$path" >&2
    exit 1
  fi
  chmod +x "$path"
}

check_param() {
  local path="$1"
  # ncnn magic. The realesrgan loader reads the blob named "data".
  # Avoid `head | grep` here: pipefail treats grep -q's early exit as a failure.
  local magic head4
  magic="$(head -c 24 "$path" || true)"
  if [[ "$magic" != *7767517* ]]; then
    echo "not an ncnn param file: $path" >&2
    head -c 200 "$path" >&2 || true
    echo >&2
    exit 1
  fi
  head4="$(head -n 4 "$path" || true)"
  if [[ "$head4" != *data* ]]; then
    echo "warning: $path does not mention the 'data' input blob on the first lines" >&2
  fi
}

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

if ! file_ok "$BIN/upscayl-bin" 1000000; then
  download "$UPSCAYL_URL" "$TMP/upscayl.zip" 1000000
  unzip -q "$TMP/upscayl.zip" -d "$TMP/upscayl"
  copy_matching "$TMP/upscayl" 'upscayl-bin' "$BIN/upscayl-bin"
fi
check_elf "$BIN/upscayl-bin"

if ! file_ok "$BIN/realesrgan-ncnn-vulkan" 1000000 || ! file_ok "$MODELS/realesrgan-x4plus.bin" 1000000; then
  download "$REALESRGAN_URL" "$TMP/realesrgan.zip" 1000000
  unzip -q "$TMP/realesrgan.zip" -d "$TMP/realesrgan"
  if ! file_ok "$BIN/realesrgan-ncnn-vulkan" 1000000; then
    copy_matching "$TMP/realesrgan" 'realesrgan-ncnn-vulkan' "$BIN/realesrgan-ncnn-vulkan"
  fi
  if ! file_ok "$MODELS/realesrgan-x4plus.param" 1000; then
    copy_matching "$TMP/realesrgan" 'realesrgan-x4plus.param' "$MODELS/realesrgan-x4plus.param"
  fi
  if ! file_ok "$MODELS/realesrgan-x4plus.bin" 1000000; then
    copy_matching "$TMP/realesrgan" 'realesrgan-x4plus.bin' "$MODELS/realesrgan-x4plus.bin"
  fi
fi
check_elf "$BIN/realesrgan-ncnn-vulkan"
check_param "$MODELS/realesrgan-x4plus.param"

download "$NOMOS_PARAM_URL" "$MODELS/4xNomos8kSC.param" 1000
download "$NOMOS_BIN_URL" "$MODELS/4xNomos8kSC.bin" 1000000
check_param "$MODELS/4xNomos8kSC.param"

download "$ULTRA_PARAM_URL" "$MODELS/ultrasharp-4x.param" 1000
download "$ULTRA_BIN_URL" "$MODELS/ultrasharp-4x.bin" 1000000
check_param "$MODELS/ultrasharp-4x.param"

if ! file_ok "$SAMPLES/baboon.jpg" 5000; then
  download "$BABOON_URL" "$SAMPLES/baboon-full.png" 5000
  # Small enough for an iGPU timing run, still a real animal photograph.
  convert "$SAMPLES/baboon-full.png" -resize 320x -quality 92 "$SAMPLES/baboon.jpg"
fi

# 64×64 stand-in so the Vulkan check does not depend on the photo download.
convert -size 64x64 plasma: "$SAMPLES/smoke.png"

log "Vulkan devices"
vulkaninfo --summary | awk '
  /^GPU[0-9]+:/ { gpu=$0 }
  /deviceName/ { name=$0; sub(/^[[:space:]]*deviceName[[:space:]]*=[[:space:]]*/, "", name); print gpu, name }
  /deviceType/ { kind=$0; sub(/^[[:space:]]*deviceType[[:space:]]*=[[:space:]]*/, "", kind); print "   ", kind }
'

SMOKE_LOG="$SAMPLES/out/smoke.log"
SMOKE_OUT="$SAMPLES/out/smoke-up.png"
rm -f "$SMOKE_OUT"
log "Vulkan smoke test (realesrgan-x4plus, 64×64, tile 64)"
set +e
timeout 180 "$BIN/upscayl-bin" \
  -i "$SAMPLES/smoke.png" \
  -o "$SMOKE_OUT" \
  -m "$MODELS" \
  -n realesrgan-x4plus \
  -z 4 \
  -t 64 \
  -j 1:1:1 \
  -f png \
  >"$SMOKE_LOG" 2>&1
code=$?
set -e
if [[ "$code" -ne 0 ]] || [[ ! -s "$SMOKE_OUT" ]]; then
  echo "smoke test failed (exit $code)" >&2
  cat "$SMOKE_LOG" >&2 || true
  exit 1
fi
log "smoke test wrote $SMOKE_OUT"
tail -n 5 "$SMOKE_LOG" || true

log "ready"
printf '  binary  %s\n' "$BIN/upscayl-bin"
printf '  cpu     %s\n' "$BIN/realesrgan-ncnn-vulkan"
printf '  models  %s\n' "$MODELS"
ls -1 "$MODELS"/*.param
printf '  sample  %s\n' "$SAMPLES/baboon.jpg"
