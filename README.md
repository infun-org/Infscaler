# InfScaler

Local photo enhancement for Linux. The window is an egui app. Two backends
share that window:

- **NCNN-Vulkan**, a subprocess (`bin/upscayl-bin`) with tiling and optional
  TTA, the same shape as [FreeScaler](https://github.com/TheMurusTeam/FreeScaler).
  This path loads `.param` + `.bin` RRDB models and uses the Radeon 660M.
- **ONNX Runtime on the CPU**, via the `ort` crate. This is the working CPU
  path (the NCNN binaries reject `-g -1`) and the way to run SPAN, DAT, and
  SCUNet, which that NCNN binary cannot load. DRCT-L is listed as BROKEN and
  disabled because tiled output retains a square grid. Tiles and overlap are blended
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
./scripts/setup_face_models.sh        # optional YuNet + RestoreFormer++ face models
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

The window can open a photo (file dialog or drag and drop), pick a model or **Auto**,
scale (2×, 3×, 4×), tile size (`0` = auto), overlap, GPU id, TTA, keep-original-size,
and an optional **Denoise first (SCUNet)** pass. Enhance runs on a background
thread. The before/after view is a drag slider. Errors are shown under the
progress bar, with elapsed time.

Face restoration is enabled by default. YuNet detects faces in the enhanced
image, aligns each crop using its eye landmarks, restores at 512×512 with
RestoreFormer++, then pastes it back with a feathered mask. The strength slider
controls the blend. Install the optional face ONNX files with
`scripts/setup_face_models.sh`; when either file is missing, the face step is
skipped and reported in status. Animal photos without detected faces skip it.
Auto selects RealWebPhoto DAT for images with faces and Nomos2 OTF otherwise.
**Fast-ish Auto** selects Nomos2 OTF for portraits, and **Fast Auto** selects
Nomos8kSC on Vulkan. Manual model selection remains
available. Status and logs report per-step times and the model Auto chose.

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

./target/release/infscaler \
  --input samples/real/portrait.jpg \
  --output samples/out/portrait-auto.png \
  --model auto \
  --face-strength 0.7
```

| Flag | Meaning |
| --- | --- |
| `--input` | Photo. If omitted, the GUI opens |
| `--output` | `.png`, `.jpg`, or `.webp` |
| `--model` | File stem in `models/`, or `auto` (default) |
| `--fast` | In Auto mode, use Nomos8kSC on the Vulkan GPU |
| `--fast-ish` | In Auto mode, use Nomos2 OTF for portraits |
| `--face-restore [model]` | Use the optional ONNX face restorer (enabled by default; default model `RestoreFormerPlusPlus`) |
| `--no-face-restore` | Disable the face restore step |
| `--face-strength` | Face paste-back blend from 0 to 1 (default 0.8) |
| `--scale` | 2, 3, or 4. Default 4. Ignored with `--keep-size` |
| `--tile` | Tile size. `0` is 128. A graph with a fixed square input (DAT, SCUNet) always uses that size |
| `--overlap` | ONNX seam overlap in input pixels. Default 16. DAT and SCUNet use at least 48; SPAN at least 32. NCNN ignores it |
| `--tta` | Eight-orientation test-time augmentation. NCNN only |
| `--keep-size` | Native model scale, then Lanczos3 back to the original dimensions |
| `--denoise` | Run the SCUNet ONNX model first, then the selected model |
| `--gpu` | `auto` (default) or a device id such as `0`. `cpu` is rejected for NCNN |
| `--list-models` | Print `name`, backend, scale, role, sha256. No window |

```bash
./target/release/infscaler --list-models
./scripts/bench.sh    # every model, keep-size on and off, samples/out/bench/
./scripts/bench5.sh   # Auto + face pipeline, samples/out/bench5/
```

NCNN is started with `-j 1:1:1`, so the integrated GPU does not run two copies
of the network at once. ONNX Runtime uses all CPU threads inside one process.

## How a run works

1. The file extension picks the backend. `.param` + `.bin` starts `bin/upscayl-bin` (or `bin/realesrgan-ncnn-vulkan` if that binary is missing and the model is an official Real-ESRGAN name). `.onnx` loads ONNX Runtime.
2. With `--denoise`, SCUNet runs first into a temporary PNG, then the selected model. If the selected model is already SCUNet, it runs once.
3. The image is enhanced at the model's native scale. NCNN tiles and TTA are flags on that process (`-t`, `-x`). ONNX tiles are cut in Rust, run as RGB float32 NCHW in 0–1, reflect-padded to the tile, and blended with a Hann ramp across the overlap.
4. If the requested size is not the native output (another scale, or keep-original-size), InfScaler resizes with Lanczos3.
5. Progress from the NCNN log, or from finished ONNX tiles, drives the bar. The CLI prints elapsed time and peak `VmHWM`.

## Limits

- No discrete GPU required, but RRDB models are heavy on a 660M. A 320 px photo is seconds to a couple of minutes per model. TTA multiplies that by about eight. Large tiles can exhaust shared memory (`VK_ERROR_OUT_OF_DEVICE_MEMORY`); lower the tile size.
- Keep-original-size decodes the full 4× image before the Lanczos pass. A big photo can use a lot of RAM on a 13 GB machine.
- Neither shipped NCNN binary accepts CPU mode (`-g -1`). ONNX Runtime is the CPU path.
- DAT and SCUNet ONNX files use a fixed 128×128 input. The runtime reflect-pads every tile to that size and blends at least 48 px of overlap. DRCT-L is listed as BROKEN and disabled: its fixed-128 tile matches PyTorch, but tiled full-photo output still has severe square-grid artifacts. Wider overlap is too slow, while full-frame inference cannot fit inside the 7 GiB process limit with safe margin.
- The param path must be shorter than 256 bytes. That is a fixed buffer in the C binary.
- Real-photo sources and licenses are in `samples/real/SOURCES.md`. Benchmark crops land in `samples/out/bench/compare/`.
