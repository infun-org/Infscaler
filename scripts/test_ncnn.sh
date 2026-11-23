#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="${NCNN_BIN:-$ROOT/bin/upscayl-bin}"
INPUT="${NCNN_INPUT:-$ROOT/samples/baboon.jpg}"
MODEL="${NCNN_MODEL:-4xNomos8kSC}"
GPU="${NCNN_GPU:-0}"
OUT="$ROOT/samples/out/ncnn-tests"
mkdir -p "$OUT"

if [[ ! -x "$BIN" ]]; then echo "NCNN binary not executable: $BIN" >&2; exit 1; fi
if [[ ! -f "$INPUT" ]]; then echo "Input image missing: $INPUT" >&2; exit 1; fi
if [[ ! -f "$ROOT/models/$MODEL.param" || ! -f "$ROOT/models/$MODEL.bin" ]]; then echo "Model pair missing: models/$MODEL.param/.bin" >&2; exit 1; fi

echo "Binary: $BIN"
echo "Input: $INPUT"
echo "Model: $MODEL, GPU $GPU"
echo "Variant,mean_rgb,output"
for tile in 0 128 256; do
  for jobs in 1:1:1 2:2:2; do
    for fp16 in on off; do
      tag="t${tile}_j${jobs//:/-}_fp16-${fp16}"
      output="$OUT/$tag.png"
      args=(-i "$INPUT" -o "$output" -m "$ROOT/models" -n "$MODEL" -z 4 -t "$tile" -j "$jobs" -g "$GPU" -f png)
      if [[ "$fp16" == off ]]; then args+=(-fp16 0); fi
      rm -f "$output"
      if "$BIN" "${args[@]}" >"$OUT/$tag.log" 2>&1 && [[ -s "$output" ]]; then
        if command -v magick >/dev/null; then mean=$(magick "$output" -format '%[fx:mean]' info:)
        elif command -v identify >/dev/null; then mean=$(identify -format '%[fx:mean]' "$output")
        else mean="(install ImageMagick for mean)"; fi
        printf '%s,%s,%s\n' "$tag" "$mean" "$output"
      else
        printf '%s,FAILED,%s\n' "$tag" "$OUT/$tag.log"
      fi
    done
  done
done
