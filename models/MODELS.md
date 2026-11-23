# InfScaler models

InfScaler does not implement a neural network. It runs a prebuilt NCNN-Vulkan
binary on a `<name>.param` + `<name>.bin` pair in this folder, the same way
[FreeScaler](https://github.com/TheMurusTeam/FreeScaler) shells out to
Real-ESRGAN. The app lists every pair it finds here. `manifest.json` supplies
scale, license, and source when the file name is known; otherwise the scale is
inferred from `2x` / `x2`, `3x` / `x3`, `4x` / `x4` in the name (default 4).

`scripts/setup.sh` downloads the three models below. Weights are not committed
(see `.gitignore`). Re-run the script to fetch them.

The model directory **must** be named `models`. upscayl-bin checks that the
path contains `models` and then sets the Real-ESRGAN prepadding to 10. The
param path must also fit in the binary's 256-byte buffer.

## realesrgan-x4plus

| | |
| --- | --- |
| Scale | 4 |
| Architecture | RRDB / Real-ESRGAN |
| License | BSD-3-Clause (Real-ESRGAN, Copyright 2021 Xintao Wang) |
| Role | Baseline photo model, the one FreeScaler-style apps ship |
| NCNN files | Inside the Ubuntu release zip below (`models/realesrgan-x4plus.param`, `.bin`) |
| Source | https://github.com/xinntao/Real-ESRGAN/releases/download/v0.2.5.0/realesrgan-ncnn-vulkan-20220424-ubuntu.zip |
| Upstream | https://github.com/xinntao/Real-ESRGAN |

## 4xNomos8kSC

| | |
| --- | --- |
| Scale | 4 |
| Architecture | ESRGAN RRDBNet, 64 filters, 23 blocks. Pretrained from RealESRGAN_x4plus |
| Author | Philip Hofmann (Phhofm / Helaman) |
| Released | 2023-05-10 |
| License | CC-BY-4.0 |
| Role | Newer photo model. On-the-fly JPEG and blur degradation, aimed at real photographs rather than illustration |
| NCNN files | `4xNomos8kSC.param`, `4xNomos8kSC.bin` |
| NCNN source | https://github.com/upscayl/custom-models/tree/main/models |
| Original weights | https://github.com/Phhofm/models/releases/tag/4xNomos8kSC |
| Card | https://openmodeldb.info/models/4x-Nomos8kSC |

This is the default model in the GUI. It is the practical choice for people
and animals when the source is a bit soft or JPEG-compressed.

## ultrasharp-4x

| | |
| --- | --- |
| Scale | 4 |
| Architecture | ESRGAN |
| Author | Kim2091 |
| Released | 2021-10-27 |
| License | CC-BY-NC-SA-4.0 (non-commercial, credit required, share-alike) |
| Role | Strong sharpening and texture synthesis (fur, hair, fabric). Can look harsh on skin |
| NCNN files | `ultrasharp-4x.param`, `ultrasharp-4x.bin` (Upscayl's converted copy) |
| NCNN source | https://github.com/upscayl/upscayl/tree/main/resources/models |
| Card | https://openmodeldb.info/models/4x-UltraSharp |

## Why these, and not SPAN / DRCT / DAT / 4xNomos2

upscayl-bin is a fork of Real-ESRGAN-ncnn-vulkan. It runs an RRDB graph
(convolution, concat, eltwise, plus the realesrgan pre/post shaders). It does
not ship SPAN, DRCT, DAT, HAT, or SwinIR operators.

pnnx, the NCNN PyTorch converter, does not produce a runnable graph for those
transformers. Window attention, relative position bias, and several LayerNorm
variants have no Vulkan implementation in this binary, so a converted
`.param` would fail at load even if pnnx exited 0. InfScaler therefore does
not try to convert DRCT or DAT. SPAN has its own runtime (`span-ncnn-vulkan`)
with a different command line; wiring a second engine is outside this
FreeScaler-style design.

`4xNomos2_otf_esrgan` (2024, also by Philip Hofmann) is a newer photo ESRGAN
and is convertible in principle, but there is no maintained NCNN pair already
rewritten for this loader (the input blob has to be named `data`, not
`input`). Doing that conversion needs a full PyTorch + chaiNNer/pnnx setup,
which is a poor fit for a 13 GB machine with no discrete GPU, and it is easy
to emit a graph this binary will reject. `4xNomos8kSC` is the same author's
photo ESRGAN, already published as a working Upscayl NCNN pair, trained for
blur and JPEG. That is the newer model this app ships.

## Backends

| Binary | File | License | Role |
| --- | --- | --- | --- |
| upscayl-bin 20251207-174704 | `bin/upscayl-bin` | AGPL-3.0 | Primary. Arbitrary ESRGAN model names, tiling, TTA. GPU only: a negative `-g` is rejected |
| realesrgan-ncnn-vulkan 20220424 | `bin/realesrgan-ncnn-vulkan` | BSD-3-Clause | Fallback if the upscayl download fails. Loads only official Real-ESRGAN names (`realesrgan-x4plus`, `realesrnet-x4plus`, `realesrgan-x4plus-anime`, `realesr-animevideov3`). `-g -1` was tested and returns `invalid gpu device`, so this build is not a CPU path either |

Both are fetched by `scripts/setup.sh` and are not committed.

- upscayl-ncnn release: https://github.com/upscayl/upscayl-ncnn/releases/tag/20251207-174704
- Real-ESRGAN release: https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0
