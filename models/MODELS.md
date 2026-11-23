# InfScaler models

InfScaler does not implement a neural network. It runs one of two backends:

- a prebuilt NCNN-Vulkan binary on a `<name>.param` + `<name>.bin` pair
- ONNX Runtime on the CPU for a `<name>.onnx` file

The app lists every pair and every ONNX file it finds here. If the same stem
has both, the NCNN pair is used. `manifest.json` supplies scale, license,
source, sha256, and a denoise flag. Otherwise the scale is inferred from
`2x` / `x2`, `3x` / `x3`, `4x` / `x4` in the name (default 4). A name
containing `scunet` is scale 1 and is the denoise pre-step.

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

This is the NCNN default when the 2024 ONNX model below is not installed.
It is the practical Vulkan choice for people and animals when the source is a
bit soft or JPEG-compressed.

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

## Why NCNN does not load SPAN / DRCT / DAT

upscayl-bin is a fork of Real-ESRGAN-ncnn-vulkan. It runs an RRDB graph
(convolution, concat, eltwise, plus the realesrgan pre/post shaders). It does
not ship SPAN, DRCT, DAT, or SCUNet operators. Those models are ONNX files,
run by ONNX Runtime on the CPU. `scripts/export_models.sh` reproduces the
export (CPU PyTorch, spandrel, opset 17). Weights and ONNX files are
gitignored. sha256 below is of the ONNX file.

ONNX tiles are RGB float32 NCHW in 0–1. DAT, DRCT, and SCUNet are forced to a
64×64 tile: the trace constant-folds window attention, and a 160 px SCUNet
tile fails at runtime. SPAN and the ESRGAN ONNX accept other tile sizes.

## 4xNomos2_otf_esrgan

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | ESRGAN |
| Author | Philip Hofmann |
| License | CC-BY-4.0 |
| Role | General photo model. On-the-fly JPEG and blur. Preferred GUI default when the file is present |
| ONNX sha256 | `3c604b8878683b448aaa3a5736386e45e630dff295a38b2b57e8ab29812da90d` |
| Weights | https://github.com/Phhofm/models/releases/download/4xNomos2_otf_esrgan/4xNomos2_otf_esrgan.safetensors |
| Card | https://openmodeldb.info/models/4x-Nomos2-otf-esrgan |
| Export | spandrel, opset 17, dynamic H/W. Input `input`, output `output` |

The author also publishes `4xNomos2_otf_esrgan_fp32_opset17.onnx`. The file in
`models/` is the local spandrel export, not that download.

## spanx4_ch48

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | SPAN, 48 channels |
| Author | Hongyuan Yu and the SPAN authors |
| License | Apache-2.0 |
| Role | Light, faithful 4× photo model |
| ONNX sha256 | `f28af5716a997e9881b6a0541f1ee07852702370bbc78852da746fea1e14f670` |
| Weights | https://github.com/OpenModelDB/model-hub/releases/download/span/4x-spanx4_ch48.pth |
| Card | https://openmodeldb.info/models/4x-spanx4-ch48 |
| Export | spandrel, opset 17, dynamic H/W. Checked at tile 64 and tile 128 |

## 4xRealWebPhoto_v4_dat2

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | DAT-2 |
| Author | Philip Hofmann |
| License | CC-BY-4.0 |
| Role | JPEG and web-compressed photos |
| ONNX sha256 | `c602ba42c58c3e5394b52ade953b9e1448f9eff2e2d9bcd061b7b81a84a2d340` |
| Weights | https://github.com/Phhofm/models/releases/download/4xRealWebPhoto_v4_dat2/4xRealWebPhoto_v4_dat2.safetensors |
| Card | https://openmodeldb.info/models/4x-RealWebPhoto-v4-dat2 |
| Export | spandrel, opset 17. The graph names H/W as dynamic, and the trace also folds spatial asserts, so InfScaler always uses a 64 tile. A 96×96 crop, four 64 tiles, took about 15 s and peaked near 380 MB RSS |

The author also publishes `4xRealWebPhoto_v4_dat2_fp32_opset17.onnx`. The file
in `models/` is the local export.

## scunet_color_real_psnr

| | |
| --- | --- |
| Scale | 1 |
| Backend | ONNX Runtime CPU |
| Architecture | SCUNet |
| Author | Kai Zhang, Yawei Li, Jingyun Liang, Jiezhang Cao, Yulun Zhang, Hao Tang, Deng-Ping Fan, Radu Timofte, Luc Van Gool |
| License | Apache-2.0 (SCUNet repository, Copyright 2022 Kai Zhang). The checkpoint is the file the SCUNet README tells you to run |
| Role | Optional "Denoise first" pre-step. Real-image blind denoise, PSNR checkpoint. Not a scale model |
| ONNX sha256 | `3767d2ebf20a2f839e7b88349af98e6b23682aac645d911c4dac0d2f4f5f00e8` |
| Weights | https://github.com/cszn/KAIR/releases/download/v1.0/scunet_color_real_psnr.pth |
| Code | https://github.com/cszn/SCUNet |
| Export | spandrel, opset 17. A 64 tile runs. A 160 tile fails in the window-attention reshape, because that shape was constant-folded at export. InfScaler forces tile 64 |

## 4xNomos2_hq_drct-l

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | DRCT-L |
| Author | Philip Hofmann |
| License | CC-BY-4.0 |
| Role | Large photo model. Included because a 64 tile fits: a 96×96 crop peaked at about 450 MB RSS and finished in about 12 s |
| ONNX sha256 | `1491656456a1c231643fdb178c9d0261e5742be119f71dd33eb0333c84655d26` |
| Weights | https://github.com/Phhofm/models/releases/download/4xNomos2_hq_drct-l/4xNomos2_hq_drct-l.safetensors |
| Export | spandrel, opset 17. The first combined run skipped it (2.6 GiB free, under the 4 GiB guard). A later `INFSCALER_ONLY=4xNomos2_hq_drct-l` run exported it. Same 64-tile limit as DAT. No author ONNX fallback exists |

The guard remains in `scripts/export_models.py` so a low-RAM re-run of every
model does not start this one. Set `INFSCALER_ONLY` to export it on its own.

## Backends

| Binary | File | License | Role |
| --- | --- | --- | --- |
| upscayl-bin 20251207-174704 | `bin/upscayl-bin` | AGPL-3.0 | Primary. Arbitrary ESRGAN model names, tiling, TTA. GPU only: a negative `-g` is rejected |
| realesrgan-ncnn-vulkan 20220424 | `bin/realesrgan-ncnn-vulkan` | BSD-3-Clause | Fallback if the upscayl download fails. Loads only official Real-ESRGAN names (`realesrgan-x4plus`, `realesrnet-x4plus`, `realesrgan-x4plus-anime`, `realesr-animevideov3`). `-g -1` was tested and returns `invalid gpu device`, so this build is not a CPU path either |

Both are fetched by `scripts/setup.sh` and are not committed.

- upscayl-ncnn release: https://github.com/upscayl/upscayl-ncnn/releases/tag/20251207-174704
- Real-ESRGAN release: https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0
