# InfScaler

Local photo enhancement for Linux. The window is an egui app. Two backends
share that window:

- **NCNN-Vulkan**, a subprocess (`bin/upscayl-bin`) with tiling and optional
  TTA, the same shape as [FreeScaler](https://github.com/TheMurusTeam/FreeScaler).
  This path loads `.param` + `.bin` RRDB models and uses the Radeon 660M.
- **ONNX Runtime on the CPU**, via the `ort` crate. This is the working CPU
  path (the NCNN binaries reject `-g -1`) and the way to run SPAN, DAT, DRCT,
  and SCUNet, which that NCNN binary cannot load. Tiles and overlap are blended
  in Rust. The scale comes from the output shape.

Aimed at real photos of people and animals: recovering soft detail matters
more than making a picture enormous. **Keep original size** runs the model at
its native scale, then downscales with a 3-lobe Lanczos filter back to the
original pixel size.

Tested on an AMD Ryzen 5 7535H with Radeon 660M (RADV, integrated, no discrete
GPU).

## What you need

- Rust (this tree builds with 1.100; edition 2021, rust-version 1.92)
- A Vulkan GPU for the NCNN models. Integrated RADV is enough. `vulkan-radeon` and `vulkan-tools` on Arch
- Build libraries eframe uses on Linux: `base-devel`, `libxkbcommon`, `libxcb`, GTK 3 (the file dialog), OpenSSL (ONNX Runtime download)
- `curl`, `unzip`, and ImageMagick (`magick`) for `scripts/setup.sh`
- Python 3.12 for `scripts/export_models.sh` and `scripts/fetch_real_photos.py`

On Arch / CachyOS:

```bash
sudo pacman -S --needed base-devel curl unzip imagemagick \
  vulkan-radeon vulkan-tools libxkbcommon gtk3
```

## Setup

From the repo root:

```bash
./scripts/setup.sh
```

That downloads:

- `bin/upscayl-bin` from the [upscayl-ncnn](https://github.com/upscayl/upscayl-ncnn) release `20251207-174704` (arbitrary ESRGAN model names, GPU only)
- `bin/realesrgan-ncnn-vulkan` from the [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN) `v0.2.5.0` Ubuntu build, a GPU fallback if `upscayl-bin` is missing (official model names only; not a CPU path)
- `models/realesrgan-x4plus`, `models/4xNomos8kSC`, and `models/ultrasharp-4x`
- `samples/baboon.jpg`, a small mandrill photo

The script then runs a 64×64 upscale so you know the binary actually talks to
the Vulkan device. Weights and binaries are gitignored. Licenses and source
URLs are in `models/MODELS.md`.

ONNX models are a second step. `scripts/export_models.sh` creates `.venv`,
installs a CPU build of PyTorch and [spandrel](https://github.com/chaiNNer-org/spandrel),
and writes `models/<name>.onnx` (opset 17, dynamic height and width). If a
local export fails and the author published an opset-17 ONNX, that file is
used instead. The report is `models/export-report.json`.

```bash
./scripts/export_models.sh
python3 scripts/fetch_real_photos.py   # samples/real/, with SOURCES.md
```

A `.param` + `.bin` pair is the NCNN backend. A `.onnx` file is ONNX Runtime.
If both exist for the same stem, the NCNN pair wins. The model menu shows the
backend and the scale. Drop extra files into `models/` and they show up. The
folder has to stay named `models` for the NCNN binary.

## Build and launch the window

```bash
cargo build --release
./target/release/infscaler
```

`cargo run --release` also works. The app finds the repo by walking up from
the executable and the current directory until it sees `scripts/setup.sh`.
If you move the binary, set `INFSCALER_HOME` to the repo root.

The window can open a photo (file dialog or drag and drop), pick a model,
scale (2×, 3×, 4×), tile size (`0` = auto), overlap, GPU id, TTA, keep-original-size,
and an optional **Denoise first (SCUNet)** pass. Enhance runs on a background
thread. The before/after view is a drag slider. Errors are shown under the
progress bar, with elapsed time.

The GPU list is filled from `vulkaninfo`. On this machine that is device 0,
AMD Radeon 660M. Choosing CPU for an NCNN model fails: upscayl-bin rejects a
negative GPU id, and the 20220424 `realesrgan-ncnn-vulkan` build returns
`invalid gpu device` for `-g -1`. ONNX models ignore the GPU id and run on the
CPU with every hardware thread (`available_parallelism`, 12 on this machine).
TTA is NCNN-only. The official binary is still installed as a fallback if
`upscayl-bin` is missing; it only loads Real-ESRGAN's own model names.

## CLI

```bash
./target/release/infscaler \
  --input samples/baboon.jpg \
  --output samples/out/baboon-nomos.png \
  --model 4xNomos8kSC \
  --tile 128

./target/release/infscaler \
  --input samples/baboon.jpg \
  --output samples/out/baboon-keep.png \
  --model realesrgan-x4plus \
  --keep-size \
  --tile 128 \
  --tta
```

| Flag | Meaning |
| --- | --- |
| `--input` | Photo. If omitted, the GUI opens |
| `--output` | `.png`, `.jpg`, or `.webp` |
| `--model` | File stem in `models/` |
| `--scale` | 2, 3, or 4. Default 4. Ignored with `--keep-size` |
| `--tile` | Tile size. `0` is auto (64 for DAT/DRCT, 192 for SCUNet, else 128) |
| `--overlap` | ONNX seam overlap in input pixels. Default 16. NCNN ignores it |
| `--tta` | Eight-orientation test-time augmentation. NCNN only |
| `--keep-size` | Native model scale, then Lanczos3 back to the original dimensions |
| `--denoise` | Run the SCUNet ONNX model first, then the selected model |
| `--gpu` | `auto` (default) or a device id such as `0`. `cpu` is rejected for NCNN |
| `--list-models` | Print `name`, backend, scale, role, sha256. No window |

```bash
./target/release/infscaler --list-models
./scripts/bench.sh    # every model, keep-size on and off, samples/out/bench/
```

NCNN is started with `-j 1:1:1`, so the integrated GPU does not run two copies
of the network at once. ONNX Runtime uses all CPU threads inside one process.

## How a run works

1. The file extension picks the backend. `.param` + `.bin` starts `bin/upscayl-bin` (or `bin/realesrgan-ncnn-vulkan` if that binary is missing and the model is an official Real-ESRGAN name). `.onnx` loads ONNX Runtime.
2. With `--denoise`, SCUNet runs first into a temporary PNG, then the selected model. If the selected model is already SCUNet, it runs once.
3. The image is enhanced at the model's native scale. NCNN tiles and TTA are flags on that process (`-t`, `-x`). ONNX tiles are cut in Rust, run as RGB float32 NCHW in 0–1, and blended with a linear ramp across the overlap.
4. If the requested size is not the native output (another scale, or keep-original-size), InfScaler resizes with Lanczos3.
5. Progress from the NCNN log, or from finished ONNX tiles, drives the bar. The CLI prints elapsed time and peak `VmHWM`.

## Limits

- No discrete GPU required, but RRDB models are heavy on a 660M. A 320 px photo is seconds to a couple of minutes per model. TTA multiplies that by about eight. Large tiles can exhaust shared memory (`VK_ERROR_OUT_OF_DEVICE_MEMORY`); lower the tile size.
- Keep-original-size decodes the full 4× image before the Lanczos pass. A big photo can use a lot of RAM on a 13 GB machine.
- Neither shipped NCNN binary accepts CPU mode (`-g -1`). ONNX Runtime is the CPU path.
- DAT, DRCT, and SCUNet use a 64 px tile when tile is 0. Those ONNX graphs were traced at 64×64, so a larger tile fails inside window attention. Overlap is 16 px unless you change it.
- The param path must be shorter than 256 bytes. That is a fixed buffer in the C binary.
- Real-photo sources and licenses are in `samples/real/SOURCES.md`. Benchmark crops land in `samples/out/bench/compare/`.
