# InfScaler models

InfScaler does not implement a neural network. It runs one of two backends:

- a prebuilt NCNN-Vulkan binary on a `<name>.param` + `<name>.bin` pair
- ONNX Runtime on the CPU for a `<name>.onnx` file

The app lists every pair and every ONNX file it finds here. A manifest entry
with `broken_reason` remains visible as BROKEN but cannot be selected or run.
If the same stem
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

ONNX tiles are RGB float32 NCHW in 0–1. DAT and SCUNet are exported with a
fixed 128×128 input and no dynamic axes; the runtime reads the static input
size, reflect-pads every tile to it, and keeps at least 48 px of overlap with
a Hann ramp. SPAN and the ESRGAN ONNX stay dynamic; SPAN keeps at least 32 px
of overlap. SPAN's export mirrors InfScaler's final output clamp, and its
PyTorch parity check uses a real photo crop because random noise is outside
the model's useful input distribution.

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
| ONNX sha256 | `966008422ac62fffed2795c662b4f1b10905886ccb0ec7e30b4cd5cebf4e1ac6` |
| Weights | https://github.com/OpenModelDB/model-hub/releases/download/span/4x-spanx4_ch48.pth |
| Card | https://openmodeldb.info/models/4x-spanx4-ch48 |
| Export | spandrel, opset 17, dynamic H/W. Checks the natural-image 0–1 output range on a real crop and mirrors InfScaler's output clipping. Max abs vs PyTorch on real dog-photo crops: 1.40e-6 at 64, 1.73e-6 at 128 |

## 4xRealWebPhoto_v4_dat2

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | DAT-2 |
| Author | Philip Hofmann |
| License | CC-BY-4.0 |
| Role | JPEG and web-compressed photos |
| ONNX sha256 | `68be66e0baf7fb1034c0466e62a160a09eb4e561f7f83cc102fb67b997491485` |
| Weights | https://github.com/Phhofm/models/releases/download/4xRealWebPhoto_v4_dat2/4xRealWebPhoto_v4_dat2.safetensors |
| Card | https://openmodeldb.info/models/4x-RealWebPhoto-v4-dat2 |
| Export | spandrel, opset 17, fixed 128×128 input (no dynamic axes). A dynamic trace folded the window mask at 64, so neighbor tiles disagreed with a full-frame forward. The runtime always feeds 128 and reflect-pads the overhang |

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
| ONNX sha256 | `2380d8bfb467f36ef72ff35e50623b0efd68704ba3254a06b8b8ca8b77455ebc` |
| Weights | https://github.com/cszn/KAIR/releases/download/v1.0/scunet_color_real_psnr.pth |
| Code | https://github.com/cszn/SCUNet |
| Export | spandrel, opset 17, fixed 128×128 input. The previous trace matched PyTorch at 64 and at 128 (max abs about 1e-6) and failed a 160 tile inside the window reshape. The runtime now always feeds 128 |

## 4xNomos2_hq_drct-l

| | |
| --- | --- |
| Scale | 4 |
| Backend | ONNX Runtime CPU |
| Architecture | DRCT-L |
| Author | Philip Hofmann |
| License | CC-BY-4.0 |
| Role | BROKEN / disabled. Fixed 128×128 tiled output still shows a severe square grid |
| ONNX sha256 | `3dde58973980e979d5c5e37f6b5997dd315dbde4e21ad1cab18636c836c4faee` |
| Weights | https://github.com/Phhofm/models/releases/download/4xNomos2_hq_drct-l/4xNomos2_hq_drct-l.safetensors |
| Export | spandrel, opset 17, fixed 128×128 input. The rebuilt 128-window mask matches PyTorch at max abs 1.07e-6 on a real dog-photo crop. Yet full-photo tiled output has severe square-grid artifacts with 48 px Hann overlap. Wider overlap is prohibitively slow. Full-frame inference has not been validated under the 7 GiB memory cap; a prior DRCT trace reached about 10 GiB. The manifest marks this model BROKEN and the app blocks it |

The optional DRCT export remains individually selectable with
`INFSCALER_ONLY=4xNomos2_hq_drct-l`; that does not remove the BROKEN status
or make the model selectable in InfScaler.

## Face restoration assets

These two files are optional runtime assets, not enhancement models in the
model picker. `scripts/setup_face_models.sh` downloads them into `models/`.

| Asset | Purpose | License and provenance |
| --- | --- | --- |
| `face_detection_yunet.onnx` | YuNet 2023mar, fixed 640×640 input, emits bounding boxes and five landmarks | MIT, copyright Shiqi Yu; distributed by OpenCV Zoo. SHA-256 `8f2383e4dd3cfbb4553ea8718107fc0423210dc964f9f4280604804ed2552fa4`. [OpenCV Zoo model card](https://github.com/opencv/opencv_zoo/tree/main/models/face_detection_yunet) |
| `RestoreFormerPlusPlus.onnx` | VisoMaster ONNX conversion of RestoreFormer++; one aligned 512×512 crop in, restored RGB crop out | Upstream RestoreFormer++ repository and checkpoint are Apache-2.0. The conversion is from VisoMaster Assets v0.1.0, SHA-256 `e5df99ed4f501be2009ed8e708f407dd26ac400c55a43a01d8c8c157bc475b3f`. [Upstream repository/license](https://github.com/wzhouxiff/RestoreFormerPlusPlus), [ONNX conversion source](https://github.com/visomaster/visomaster-assets/releases/tag/v0.1.0). |

RestoreFormer++ does not expose a fidelity parameter; InfScaler exposes blend
strength instead. The face path runs after whole-image enhancement and final
resizing, uses the upscaled YuNet eye landmarks to rotate/align crops, and
feathers each restored crop into its original coordinates. YuNet can produce
false positives on animal faces; see bench5 results before relying on Auto for
an unusual species.

GFPGAN v1.4 is not selected: its repository is Apache-2.0 except for bundled
third-party code/components, including DFDNet under CC-BY-NC-SA-4.0 and a
StyleGAN2-derived component under NVIDIA's processor-specific license. The
CodeFormer checkpoints use the S-Lab non-commercial license. See the upstream
[GFPGAN license](https://github.com/TencentARC/GFPGAN/blob/master/LICENSE) and
[CodeFormer repository](https://github.com/sczhou/CodeFormer).

## Backends

| Binary | File | License | Role |
| --- | --- | --- | --- |
| upscayl-bin 20251207-174704 | `bin/upscayl-bin` | AGPL-3.0 | Primary. Arbitrary ESRGAN model names, tiling, TTA. GPU only: a negative `-g` is rejected |
| realesrgan-ncnn-vulkan 20220424 | `bin/realesrgan-ncnn-vulkan` | BSD-3-Clause | Fallback if the upscayl download fails. Loads only official Real-ESRGAN names (`realesrgan-x4plus`, `realesrnet-x4plus`, `realesrgan-x4plus-anime`, `realesr-animevideov3`). `-g -1` was tested and returns `invalid gpu device`, so this build is not a CPU path either |

Both are fetched by `scripts/setup.sh` and are not committed.

- upscayl-ncnn release: https://github.com/upscayl/upscayl-ncnn/releases/tag/20251207-174704
- Real-ESRGAN release: https://github.com/xinntao/Real-ESRGAN/releases/tag/v0.2.5.0
