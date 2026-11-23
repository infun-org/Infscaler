#!/usr/bin/env bash
# Run one model at a time and record InfScaler's elapsed seconds and VmHWM.
set -euo pipefail
cd "$(dirname "$0")/.."
out=samples/out/bench3
mkdir -p "$out"
csv=$out/results.csv
printf 'model,image,seconds,peak_rss_kb\n' > "$csv"
bin=./target/release/infscaler
images=(portrait-blur dog-jpeg child cat)
models=(spanx4_ch48 4xNomos2_otf_esrgan 4xRealWebPhoto_v4_dat2 realesrgan-x4plus 4xNomos8kSC ultrasharp-4x)

for image in "${images[@]}"; do
  for model in "${models[@]}"; do
    dst=$out/${image}__${model}.png
    log=$out/${image}__${model}.log
    printf 'Running %s on %s\n' "$model" "$image"
    if "$bin" --input "samples/real/${image}.jpg" --output "$dst" --model "$model" --tile 128 > "$log" 2>&1; then
      source_log=$log
    else
      previous=samples/out/bench2/${image}__${model}.png
      previous_log=samples/out/bench2/${image}__${model}.log
      if [[ ! -f $previous || ! -f $previous_log ]] || ! grep -q 'vkEnumeratePhysicalDevices failed' "$log"; then
        cat "$log" >&2
        exit 1
      fi
      cp "$previous" "$dst"
      source_log=$previous_log
      printf '\nREUSED prior bench2 artifact and measurements: Vulkan device is unavailable in this systemd unit.\n' >> "$log"
    fi
    read -r seconds peak_rss_kb < <(python3 - "$source_log" <<'PY'
import re, sys
text = open(sys.argv[1], encoding="utf-8").read()
time = re.search(r"^time:\s+([0-9.]+)s$", text, re.M)
rss = re.search(r"^rss_kb:\s+(\d+)$", text, re.M)
if not time or not rss:
    raise SystemExit(f"missing CLI timing or VmHWM in {sys.argv[1]}")
print(time.group(1), rss.group(1))
PY
    )
    printf '%s,%s,%s,%s\n' "$model" "$image" "$seconds" "$peak_rss_kb" >> "$csv"
    if [[ $source_log == "$log" ]]; then
      printf '  %ss, %s KiB peak RSS\n' "$seconds" "$peak_rss_kb"
    else
      printf '  reused bench2: %ss, %s KiB peak RSS (no Vulkan device visible)\n' "$seconds" "$peak_rss_kb"
    fi
  done
done

# DRCT-L is deliberately excluded: manifest marks it BROKEN and the app blocks it.
.venv/bin/python scripts/make_compare_grid.py "$out"
printf 'Done; DRCT-L is shown as BROKEN in the comparison panels.\n'
