#!/usr/bin/env bash
# Auto/face pipeline comparison. Runs one CPU model job at a time.
set -euo pipefail
cd "$(dirname "$0")/.."
out=samples/out/bench5
mkdir -p "$out"
printf 'image,chosen_model,mode,seconds,peak_rss_kb,face_count\n' > "$out/results.csv"
bin=./target/release/infscaler
images=(portrait portrait-blur person child cat dog horse)
for image in "${images[@]}"; do
  for mode in enhance face; do
    dst="$out/${image}__${mode}.png"
    log="$out/${image}__${mode}.log"
    args=(--input "samples/real/${image}.jpg" --output "$dst" --model auto --tile 128)
    if [[ $mode == enhance ]]; then args+=(--no-face-restore); fi
    echo "Running Auto $mode on $image"
    "$bin" "${args[@]}" >"$log" 2>&1
    python3 - "$log" "$image" "$mode" "$out/results.csv" <<'PY'
import re,sys
text=open(sys.argv[1],encoding='utf-8').read()
model=re.search(r'Auto chose ([^\r\n]+)',text)
secs=re.search(r'^time:\s+([0-9.]+)s$',text,re.M)
rss=re.search(r'^rss_kb:\s+(\d+)$',text,re.M)
faces=re.search(r'Face detection: (\d+) face',text)
if not (model and secs and rss): raise SystemExit('missing model/time/rss in '+sys.argv[1])
with open(sys.argv[4],'a',encoding='utf-8') as f:
 f.write(','.join([sys.argv[2],model.group(1).split(' (')[0],sys.argv[3],secs.group(1),rss.group(1),faces.group(1) if faces else '0'])+'\n')
PY
  done
done
.venv/bin/python scripts/make_bench5_grids.py "$out"
echo "Done: $out/results.csv and comparison grids."
