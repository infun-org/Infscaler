# Samples

`baboon.jpg` is a 320 px wide JPEG made by `scripts/setup.sh` from the mandrill
test image shipped as `scripts/baboon.png` in the
[Upscayl](https://github.com/upscayl/upscayl) repository. That file is the USC
SIPI miscellaneous image 4.2.03, a standard animal photograph used for image
processing tests. The script resizes it so an integrated GPU can run every
model in a few minutes.

`real/` holds freely licensed photographs of people and animals, plus two
degraded copies. Sources and licenses are in `real/SOURCES.md`.
`scripts/fetch_real_photos.py` can download them again. `scripts/bench.sh`
writes timings to `out/bench/results.csv` and center-crop comparisons to
`out/bench/compare/`.

`out/` holds InfScaler results and is gitignored.
