# InfScaler

Local photo enhancement for Linux. The window is an egui app. Processing is a
NCNN-Vulkan upscaler run as a subprocess, with tiling and optional TTA, the
same shape as [FreeScaler](https://github.com/TheMurusTeam/FreeScaler). The
models are newer photo ESRGAN networks than the Real-ESRGAN baseline, still in
the `.param` + `.bin` form that binary loads.

Aimed at real photos of people and animals: recovering soft detail matters
more than making a picture enormous. **Keep original size** runs the model at
its native scale, then downscales with a 3-lobe Lanczos filter back to the
original pixel size.

Tested on an AMD Ryzen 5 7535H with Radeon 660M (RADV, integrated, no discrete
GPU).

## What you need

- Rust (this tree builds with 1.100; edition 2021, rust-version 1.85)
- A Vulkan GPU. Integrated RADV is enough. `vulkan-radeon` and `vulkan-tools` on Arch
- Build libraries eframe uses on Linux: `base-devel`, `libxkbcommon`, `libxcb`, GTK 3 (the file dialog)
- `curl`, `unzip`, and ImageMagick (`convert`) for `scripts/setup.sh`

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
the Vulkan device. Weights and binaries are gitignored. Licenses, source URLs,
and the reason SPAN / DRCT / DAT are not included are in `models/MODELS.md`.

Drop any other realesrgan-compatible `<name>.param` and `<name>.bin` into
`models/` and they show up in the list. The folder has to stay named `models`.

## Build and launch the window

```bash
cargo build --release
./target/release/infscaler
```

`cargo run --release` also works. The app finds the repo by walking up from
the executable and the current directory until it sees `scripts/setup.sh`.
If you move the binary, set `INFSCALER_HOME` to the repo root.

The window can open a photo (file dialog or drag and drop), pick a model,
scale (2×, 3×, 4×), tile size (`0` = auto), GPU id, TTA, and keep-original-size.
Enhance runs on a background thread. The before/after view is a drag slider.
Errors from the upscaler are shown under the progress bar, with elapsed time.

The GPU list is filled from `vulkaninfo`. On this machine that is device 0,
AMD Radeon 660M. CPU mode is not offered: upscayl-bin rejects a negative GPU
id, and the 20220424 `realesrgan-ncnn-vulkan` build returns `invalid gpu
device` for `-g -1`. `--gpu cpu` fails with that explanation. The official
binary is still installed as a fallback if `upscayl-bin` is missing; it only
loads Real-ESRGAN's own model names.

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
| `--tile` | Tile size. `0` is auto. Use 64 or 128 if the 660M runs out of memory |
| `--tta` | Eight-orientation test-time augmentation |
| `--keep-size` | Native model scale, then Lanczos3 back to the original dimensions |
| `--gpu` | `auto` (default) or a device id such as `0`. `cpu` is rejected by both shipped binaries |

On this machine the processing thread count passed to the backend is `1:1:1`,
so the integrated GPU does not run two copies of the network at once.

## How a run works

1. The GUI or CLI picks `bin/upscayl-bin` (or `bin/realesrgan-ncnn-vulkan` if that binary is missing and the model is an official Real-ESRGAN name).
2. The image is upscaled at the model's native scale. Tiles and TTA are flags on that process (`-t`, `-x`).
3. If the requested size is not the native output (another scale, or keep-original-size), InfScaler resizes with Lanczos3. The upscayl binary's own resampler has no Lanczos filter, so that step stays in the Rust process.
4. Progress lines (`12.50%`) from the upscaler's stderr drive the bar.

## Limits

- No discrete GPU required, but RRDB models are heavy on a 660M. A 320 px photo is seconds to a couple of minutes per model. TTA multiplies that by about eight. Large tiles can exhaust shared memory (`VK_ERROR_OUT_OF_DEVICE_MEMORY`); lower the tile size.
- Keep-original-size decodes the full 4× image before the Lanczos pass. A big photo can use a lot of RAM on a 13 GB machine.
- Neither shipped binary accepts CPU mode (`-g -1`). Upscaling uses the Vulkan device.
- SPAN, DRCT, and DAT are not loadable. See `models/MODELS.md`.
- The param path must be shorter than 256 bytes. That is a fixed buffer in the C binary.
