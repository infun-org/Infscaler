#!/usr/bin/env python3
"""Download photo-restoration weights and export ONNX (opset 17).

SPAN and the ESRGAN model keep dynamic H/W. DAT, DRCT, and SCUNet are traced
at a fixed 128×128 input with no dynamic axes: their window attention
specializes to the trace size, and a 64 trace does not match a full-frame
forward once the image is tiled. The Rust runtime reads that static shape
and always feeds 128, with reflect padding on the overhang.

Uses a CPU torch build and spandrel. Weights land in models/weights/ (gitignored).
ONNX files land in models/<name>.onnx (gitignored). manifest.json is updated with
the sha256 of whatever file was produced.

If spandrel export fails and the author published an opset-17 ONNX, that file is
downloaded instead and the failure is recorded in models/export-report.json.
"""

from __future__ import annotations

import hashlib
import json
import os
import shutil
import sys
import traceback
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WEIGHTS = ROOT / "models" / "weights"
MODELS = ROOT / "models"
MANIFEST = MODELS / "manifest.json"
REPORT = MODELS / "export-report.json"

UA = "InfScalerExport/0.2"

# Priority order from the task. DRCT is last and optional.
TARGETS = [
    {
        "name": "4xNomos2_otf_esrgan",
        "scale": 4,
        "license": "CC-BY-4.0",
        "author": "Philip Hofmann",
        "weights": "https://github.com/Phhofm/models/releases/download/4xNomos2_otf_esrgan/4xNomos2_otf_esrgan.safetensors",
        "fallback_onnx": "https://github.com/Phhofm/models/releases/download/4xNomos2_otf_esrgan/4xNomos2_otf_esrgan_fp32_opset17.onnx",
        "card": "https://openmodeldb.info/models/4x-Nomos2-otf-esrgan",
        "notes": "2024 ESRGAN photo model. Real-ESRGAN on-the-fly JPEG and blur degradation. Exported from the author's safetensors.",
        "dummy": 64,
        "denoise": False,
    },
    {
        "name": "spanx4_ch48",
        "scale": 4,
        "license": "Apache-2.0",
        "author": "Hongyuan Yu and the SPAN authors",
        "weights": "https://github.com/OpenModelDB/model-hub/releases/download/span/4x-spanx4_ch48.pth",
        "fallback_onnx": None,
        "card": "https://openmodeldb.info/models/4x-spanx4-ch48",
        "notes": "Official SPAN 4x pretrain, 48 channels. Light and faithful. Apache-2.0 from the SPAN release mirrored by OpenModelDB.",
        "dummy": 64,
        "denoise": False,
    },
    {
        "name": "4xRealWebPhoto_v4_dat2",
        "scale": 4,
        "license": "CC-BY-4.0",
        "author": "Philip Hofmann",
        "weights": "https://github.com/Phhofm/models/releases/download/4xRealWebPhoto_v4_dat2/4xRealWebPhoto_v4_dat2.safetensors",
        "fallback_onnx": "https://github.com/Phhofm/models/releases/download/4xRealWebPhoto_v4_dat2/4xRealWebPhoto_v4_dat2_fp32_opset17.onnx",
        "card": "https://openmodeldb.info/models/4x-RealWebPhoto-v4-dat2",
        "notes": "2024 DAT-2 model for JPEG and web-compressed photos. Fixed 128×128 input; the runtime reflect-pads every tile to that size.",
        "dummy": 128,
        "fixed": True,
        "denoise": False,
    },
    {
        "name": "scunet_color_real_psnr",
        "scale": 1,
        "license": "Apache-2.0",
        "author": "Kai Zhang et al.",
        "weights": "https://github.com/cszn/KAIR/releases/download/v1.0/scunet_color_real_psnr.pth",
        "fallback_onnx": None,
        "card": "https://github.com/cszn/SCUNet",
        "notes": "SCUNet blind real-image denoiser (PSNR checkpoint). 1x pre-step. Code Apache-2.0; weights from the KAIR v1.0 release. Fixed 128×128 input.",
        "dummy": 128,
        "fixed": True,
        "denoise": True,
    },
    {
        "name": "4xNomos2_hq_drct-l",
        "scale": 4,
        "license": "CC-BY-4.0",
        "author": "Philip Hofmann",
        "weights": "https://github.com/Phhofm/models/releases/download/4xNomos2_hq_drct-l/4xNomos2_hq_drct-l.safetensors",
        "fallback_onnx": None,
        "card": "https://github.com/Phhofm/models/releases/tag/4xNomos2_hq_drct-l",
        "notes": "Large DRCT-L photo model. Fixed 128×128 input so a tile sees the same window the trace baked in. Overlap in the app is at least 48 px.",
        "dummy": 128,
        "fixed": True,
        "denoise": False,
        "optional": True,
    },
]


def download(url: str, dest: Path) -> None:
    dest.parent.mkdir(parents=True, exist_ok=True)
    if dest.is_file() and dest.stat().st_size > 1024:
        print(f"have {dest.name}")
        return
    partial = dest.with_suffix(dest.suffix + ".partial")
    print(f"get {url}")
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    with urllib.request.urlopen(req, timeout=120) as response, partial.open("wb") as out:
        shutil.copyfileobj(response, out)
    partial.replace(dest)


def sha256(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1 << 20), b""):
            digest.update(chunk)
    return digest.hexdigest()


def load_descriptor(path: Path):
    try:
        import spandrel_extra_arches

        spandrel_extra_arches.install(ignore_duplicates=True)
    except Exception as err:  # noqa: BLE001
        print(f"spandrel_extra_arches not installed ({err})")
    from spandrel import ModelLoader

    return ModelLoader().load_from_file(str(path))


def describe_onnx(path: Path) -> str:
    import onnx

    model = onnx.load(str(path), load_external_data=False)
    parts = []
    for value in list(model.graph.input) + list(model.graph.output):
        dims = []
        for dim in value.type.tensor_type.shape.dim:
            dims.append(str(dim.dim_value) if dim.dim_value else (dim.dim_param or "?"))
        parts.append(f"{value.name}[{','.join(dims)}]")
    text = "; ".join(parts)
    print(f"  graph {text}")
    return text


def rss_mb() -> str:
    try:
        for line in Path(f"/proc/{os.getpid()}/status").read_text().splitlines():
            if line.startswith("VmRSS:"):
                return f"{int(line.split()[1]) / 1024:.0f} MiB"
    except OSError:
        pass
    return "?"


def align_drct(module, side: int):
    """Rebuild DRCT so the traced window mask is the one for `side`.

    Learned weights do not depend on img_size. The attn_mask buffer does.
    Tracing at 128 with the default img_size of 64 takes the dynamic-mask
    branch and stores a much larger graph.
    """
    hp = getattr(module, "hyperparameters", None)
    if not isinstance(hp, dict) or type(module).__name__ != "DRCT":
        return module
    if int(hp.get("img_size", side)) == side:
        return module
    from spandrel.architectures.DRCT import DRCT

    kwargs = dict(hp)
    kwargs["img_size"] = side
    fresh = DRCT(**kwargs).eval()
    state = {key: value for key, value in module.state_dict().items() if "attn_mask" not in key}
    missing, unexpected = fresh.load_state_dict(state, strict=False)
    bad = [key for key in missing if "attn_mask" not in key]
    if bad or unexpected:
        raise RuntimeError(f"DRCT reload mismatch missing={bad} unexpected={unexpected}")
    print(f"  rebuilt DRCT img_size {hp.get('img_size')} -> {side} (rss {rss_mb()})", flush=True)
    del state
    return fresh


def inference_module(entry: dict, module):
    """Match the runtime's final output clamp for SPAN."""
    if entry["name"] != "spanx4_ch48":
        return module
    import torch.nn as nn

    class SpanRangeAdapter(nn.Module):
        def __init__(self, network):
            super().__init__()
            self.network = network

        def forward(self, image):
            # SPAN's [0, 1] output is correct on natural image inputs. Clamp
            # here to make ONNX parity match the clamp in InfScaler's runtime.
            return self.network(image).clamp(0.0, 1.0)

    return SpanRangeAdapter(module).eval()


def export_onnx(module, dest: Path, dummy: int, fixed: bool) -> None:
    import torch

    module.eval()
    for param in module.parameters():
        param.requires_grad_(False)
    sample = torch.zeros(1, 3, dummy, dummy)
    lowmem = os.environ.get("INFSCALER_LOWMEM") == "1"
    kwargs = dict(
        opset_version=17,
        input_names=["input"],
        output_names=["output"],
        do_constant_folding=not lowmem,
    )
    if not fixed:
        kwargs["dynamic_axes"] = {
            "input": {2: "height", 3: "width"},
            "output": {2: "height", 3: "width"},
        }
    # A warmup forward stays in the allocator and stacks on top of the trace.
    if not lowmem:
        with torch.no_grad():
            module(sample)
    print(f"  tracing {dummy}×{dummy} (rss {rss_mb()})", flush=True)
    with torch.no_grad():
        try:
            torch.onnx.export(module, sample, str(dest), dynamo=False, **kwargs)
        except TypeError:
            torch.onnx.export(module, sample, str(dest), **kwargs)
    print(f"  traced (rss {rss_mb()})", flush=True)


def reference_outputs(module, sides: list[int], entry: dict | None = None) -> list[tuple]:
    import torch

    expected = []
    generator = torch.Generator().manual_seed(0)
    for side in sides:
        if entry and entry["name"] == "spanx4_ch48":
            import numpy as np
            from PIL import Image

            photo = Image.open(ROOT / "samples" / "real" / "dog-jpeg.jpg").convert("RGB")
            photo = photo.crop((150, 60, 150 + side, 60 + side))
            pixels = np.asarray(photo, dtype=np.float32) / 255.0
            sample = torch.from_numpy(pixels.transpose(2, 0, 1).copy()).unsqueeze(0)
        else:
            sample = torch.rand(1, 3, side, side, generator=generator)
        with torch.no_grad():
            out = module(sample).detach().float().cpu().numpy()
        expected.append((sample.numpy(), out))
    return expected


def compare_onnx(onnx_path: Path, sides: list[int], expected: list[tuple], limit: float | None = 1e-3) -> list[float]:
    """Max abs difference. Refuse to publish a graph that drifted from eager mode."""
    import numpy as np
    import onnxruntime as ort

    options = ort.SessionOptions()
    options.intra_op_num_threads = 4
    session = ort.InferenceSession(str(onnx_path), sess_options=options, providers=["CPUExecutionProvider"])
    input_name = session.get_inputs()[0].name
    diffs = []
    for side, (sample, truth) in zip(sides, expected, strict=True):
        got = session.run(None, {input_name: sample})[0]
        diff = float(np.max(np.abs(truth - got)))
        print(f"  max abs vs pytorch @ {side}: {diff:.3e}")
        if limit is not None and diff > limit:
            raise RuntimeError(f"ONNX diverges from PyTorch at {side}×{side}: max abs {diff}")
        diffs.append(diff)
    return diffs


def probe_previous(onnx_path: Path, pairs: list[tuple]) -> str:
    """Diff the file we are about to replace. One size at a time so a bad size is recorded."""
    parts = []
    for side, pair in pairs:
        try:
            diff = compare_onnx(onnx_path, [side], [pair], limit=None)[0]
            parts.append(f"{side}:{diff:.3e}")
        except Exception as err:  # noqa: BLE001
            text = f"{type(err).__name__}: {err}".replace("\n", " ")
            parts.append(f"{side}:ERROR {text[:240]}")
            print(f"  previous @ {side} failed: {text[:240]}")
    return ", ".join(parts)


def update_manifest(entry: dict, digest: str, source: str, notes: str) -> None:
    data = {}
    if MANIFEST.is_file():
        data = json.loads(MANIFEST.read_text())
    data[entry["name"]] = {
        "scale": entry["scale"],
        "license": entry["license"],
        "source": source,
        "sha256": digest,
        "notes": notes,
        "denoise": bool(entry["denoise"]),
    }
    MANIFEST.write_text(json.dumps(data, indent=2) + "\n")


def main() -> int:
    import torch

    try:
        Path(f"/proc/{os.getpid()}/oom_score_adj").write_text("1000\n")
    except OSError:
        pass
    threads = int(os.environ.get("INFSCALER_THREADS", "6"))
    torch.set_num_threads(max(1, threads))
    try:
        torch.set_num_interop_threads(1)
    except RuntimeError:
        pass
    skip_verify = os.environ.get("INFSCALER_SKIP_VERIFY") == "1"
    print(f"threads={threads} skip_verify={skip_verify} lowmem={os.environ.get('INFSCALER_LOWMEM') == '1'} rss={rss_mb()}", flush=True)
    WEIGHTS.mkdir(parents=True, exist_ok=True)
    report = []
    if REPORT.is_file():
        try:
            report = json.loads(REPORT.read_text())
        except json.JSONDecodeError:
            report = []
    only = os.environ.get("INFSCALER_ONLY", "").strip()
    for entry in TARGETS:
        name = entry["name"]
        if only and name != only:
            continue
        report = [item for item in report if item.get("name") != name]
        weight_name = Path(entry["weights"]).name
        weight_path = WEIGHTS / weight_name
        onnx_path = MODELS / f"{name}.onnx"
        item = {"name": name, "ok": False}
        if entry.get("optional") and not only:
            avail = os.sysconf("SC_PAGE_SIZE") * os.sysconf("SC_AVPHYS_PAGES")
            if avail < 4 * 1024**3:
                item["error"] = f"skipped: only {avail / 1024**3:.1f} GiB available, under 4 GiB"
                print(f"SKIP {name}: {item['error']}")
                report.append(item)
                REPORT.write_text(json.dumps(report, indent=2) + "\n")
                continue
        try:
            download(entry["weights"], weight_path)
            desc = load_descriptor(weight_path)
            module = desc.model
            print(f"{name}: arch={getattr(desc, 'architecture', '?')} scale={getattr(desc, 'scale', '?')}")
            staged = onnx_path.with_suffix(".onnx.exporting")
            if staged.exists():
                staged.unlink()
            fixed = bool(entry.get("fixed"))
            if fixed and type(module).__name__ == "DRCT":
                rebuilt = align_drct(module, int(entry["dummy"]))
                if rebuilt is not module:
                    del module
                    module = rebuilt
            module = inference_module(entry, module)
            # The descriptor keeps its own reference to the loaded module.
            del desc
            import gc

            gc.collect()
            export_onnx(module, staged, entry["dummy"], fixed)
            sides = [entry["dummy"]] if fixed else [64, 128]
            pairs: list[tuple] = []
            if not skip_verify:
                expected = reference_outputs(module, sides, entry)
                pairs = list(zip(sides, expected, strict=True))
                if onnx_path.is_file() and fixed and 64 not in sides:
                    pairs = [(64, reference_outputs(module, [64])[0]), *pairs]
            del module
            gc.collect()
            print(f"  dropped pytorch (rss {rss_mb()})", flush=True)
            diffs: list[float] = []
            if skip_verify:
                print("  skipped in-process PyTorch check", flush=True)
                diff_note = "deferred to a separate process"
            else:
                if onnx_path.is_file() and pairs:
                    previous = probe_previous(onnx_path, pairs)
                    print(f"  previous onnx max abs: {previous}")
                    item["previous_max_abs"] = previous
                # pairs for the export size are the ones whose side is in `sides`
                fresh = [(side, pair) for side, pair in pairs if side in sides]
                diffs = compare_onnx(staged, [side for side, _ in fresh], [pair for _, pair in fresh])
                diff_note = ", ".join(f"{side}:{diff:.2e}" for side, diff in zip(sides, diffs, strict=True))
            graph = describe_onnx(staged)
            wanted = f"3,{entry['dummy']},{entry['dummy']}"
            if fixed and wanted not in graph.replace(" ", ""):
                raise RuntimeError(f"expected a fixed {entry['dummy']} input, graph is {graph}")
            staged.replace(onnx_path)
            digest = sha256(onnx_path)
            source = entry["weights"]
            shape_note = f"fixed {entry['dummy']}×{entry['dummy']}" if fixed else "dynamic H/W"
            notes = (
                entry["notes"]
                + f" ONNX exported locally with spandrel (opset 17, {shape_note}). "
                + f"Max abs vs PyTorch: {diff_note}. Graph: {graph}."
            )
            update_manifest(entry, digest, source, notes)
            item.update(
                ok=True,
                sha256=digest,
                bytes=onnx_path.stat().st_size,
                via="spandrel",
                graph=graph,
                max_abs=diff_note,
            )
            print(f"exported {onnx_path.name} {digest}")
        except Exception as err:  # noqa: BLE001
            traceback.print_exc()
            item["error"] = f"{type(err).__name__}: {err}"
            staged = onnx_path.with_suffix(".onnx.exporting")
            if staged.exists():
                staged.unlink()
            # A fixed-size re-export must not be replaced by an author ONNX
            # that was traced at a different resolution.
            fallback = None if entry.get("fixed") else entry.get("fallback_onnx")
            if fallback:
                try:
                    download(fallback, onnx_path)
                    digest = sha256(onnx_path)
                    graph = describe_onnx(onnx_path)
                    notes = (
                        entry["notes"]
                        + f" Local spandrel export failed ({item['error']}). "
                        + f"Using the author's published opset-17 ONNX instead. Graph: {graph}."
                    )
                    update_manifest(entry, digest, fallback, notes)
                    item.update(ok=True, sha256=digest, bytes=onnx_path.stat().st_size, via="author-onnx", export_error=item["error"])
                    print(f"fallback {onnx_path.name} {digest}")
                except Exception as err2:  # noqa: BLE001
                    item["fallback_error"] = f"{type(err2).__name__}: {err2}"
                    print(f"FAILED {name}: {item['error']}")
            else:
                print(f"FAILED {name}: {item['error']}")
        report.append(item)
        REPORT.write_text(json.dumps(report, indent=2) + "\n")
        # Drop the module graph before the next heavy load.
        import gc

        gc.collect()
    print(REPORT)
    failed = [item["name"] for item in report if not item.get("ok") and not TARGETS_BY_NAME(item["name"]).get("optional")]
    return 1 if failed else 0


def TARGETS_BY_NAME(name: str) -> dict:
    for entry in TARGETS:
        if entry["name"] == name:
            return entry
    return {}


if __name__ == "__main__":
    try:
        sys.exit(main())
    except KeyboardInterrupt:
        sys.exit(130)
