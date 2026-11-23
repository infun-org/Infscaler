#!/usr/bin/env bash
# Run every installed model on samples/real, keep-size on and off, TTA off.
# Writes samples/out/bench/results.csv and comparison crops.
set -u
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"

bin="./target/release/infscaler"
if [[ ! -x "$bin" ]]; then
  echo "build the release binary first: cargo build --release" >&2
  exit 1
fi

mkdir -p samples/out/bench
csv="samples/out/bench/results.csv"
echo "model,backend,image,input_w,input_h,keep_size,seconds,peak_rss_kb,output,status" > "$csv"

mapfile -t images < <(find samples/real -maxdepth 1 -type f \( -iname '*.jpg' -o -iname '*.jpeg' -o -iname '*.png' -o -iname '*.webp' \) | sort)
if [[ ${#images[@]} -eq 0 ]]; then
  echo "no photos in samples/real. Run scripts/fetch_real_photos.py" >&2
  exit 1
fi

mapfile -t rows < <("$bin" --list-models)
if [[ ${#rows[@]} -eq 0 || "${rows[0]}" == "(no models)" ]]; then
  echo "no models installed" >&2
  exit 1
fi

tile_for() {
  local name="${1,,}"
  # DAT/DRCT/SCUNet graphs were traced at 64. Other ONNX and NCNN models use 128.
  if [[ "$name" == *dat* || "$name" == *drct* || "$name" == *scunet* ]]; then
    echo 64
  else
    echo 128
  fi
}

for row in "${rows[@]}"; do
  model="${row%%$'\t'*}"
  tile="$(tile_for "$model")"
  for image in "${images[@]}"; do
    stem="$(basename "${image%.*}")"
    for keep in 0 1; do
      if [[ "$keep" == 1 ]]; then
        out="samples/out/bench/${stem}__${model}__keep.png"
        extra=(--keep-size)
        keep_flag=1
      else
        out="samples/out/bench/${stem}__${model}__x.png"
        extra=()
        keep_flag=0
      fi
      log="samples/out/bench/${stem}__${model}__${keep_flag}.log"
      echo "RUN $model $stem keep=$keep tile=$tile"
      set +e
      "$bin" \
        --input "$image" \
        --output "$out" \
        --model "$model" \
        --tile "$tile" \
        --overlap 16 \
        --gpu auto \
        "${extra[@]}" \
        >"$log" 2>&1
      code=$?
      set -e
      seconds="$(awk -F'[: ]+' '/^time:/ {gsub(/s/,"",$2); print $2; exit}' "$log")"
      rss="$(awk -F'[: ]+' '/^rss_kb:/ {print $2; exit}' "$log")"
      backend="$(awk -F'backend: *' '/^backend:/ {print $2; exit}' "$log")"
      iw="$(awk -F'[()×]' '/^input:/ {gsub(/ /,"",$2); print $2; exit}' "$log")"
      ih="$(awk -F'[()×]' '/^input:/ {gsub(/ /,"",$3); print $3; exit}' "$log")"
      seconds="${seconds:-}"
      rss="${rss:-}"
      if [[ "$code" == 0 && -f "$out" ]]; then
        status=ok
      else
        status=fail
        echo "  failed (exit $code), see $log" >&2
      fi
      # CSV fields that may contain commas are quoted.
      printf '%s,"%s",%s,%s,%s,%s,%s,%s,%s,%s\n' \
        "$model" "${backend//\"/\"\"}" "$stem" "${iw:-}" "${ih:-}" "$keep_flag" "${seconds:-}" "${rss:-}" "$out" "$status" \
        >> "$csv"
    done
  done
done

if [[ -x .venv/bin/python ]]; then
  .venv/bin/python scripts/compare_bench.py || echo "comparison step failed" >&2
else
  python3 scripts/compare_bench.py || echo "comparison step failed (install pillow)" >&2
fi

echo "wrote $csv"
