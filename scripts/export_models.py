#!/usr/bin/env python3
"""Download photo-restoration weights and export ONNX (opset 17, dynamic H/W).

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
        "notes": "2024 DAT-2 model for JPEG and web-compressed photos.",
        "dummy": 64,
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
        "notes": "SCUNet blind real-image denoiser (PSNR checkpoint). 1x pre-step. Code Apache-2.0; weights from the KAIR v1.0 release.",
        "dummy": 64,
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
        "notes": "Large DRCT-L photo model. Kept only if export and a small tiled run fit in 13 GB.",
        "dummy": 64,
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


def export_onnx(module, dest: Path, dummy: int) -> None:
    import torch

    module.eval()
    sample = torch.zeros(1, 3, dummy, dummy)
    with torch.no_grad():
        module(sample)
    kwargs = dict(
        opset_version=17,
        input_names=["input"],
        output_names=["output"],
        dynamic_axes={
            "input": {2: "height", 3: "width"},
            "output": {2: "height", 3: "width"},
        },
    )
    try:
        torch.onnx.export(module, sample, str(dest), dynamo=False, **kwargs)
    except TypeError:
        torch.onnx.export(module, sample, str(dest), **kwargs)


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

    torch.set_num_threads(min(8, torch.get_num_threads()))
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
            export_onnx(module, staged, entry["dummy"])
            staged.replace(onnx_path)
            digest = sha256(onnx_path)
            graph = describe_onnx(onnx_path)
            source = entry["weights"]
            notes = entry["notes"] + f" ONNX exported locally with spandrel (opset 17, dynamic H/W). Graph: {graph}."
            update_manifest(entry, digest, source, notes)
            item.update(ok=True, sha256=digest, bytes=onnx_path.stat().st_size, via="spandrel", graph=graph)
            print(f"exported {onnx_path.name} {digest}")
            del desc, module
        except Exception as err:  # noqa: BLE001
            traceback.print_exc()
            item["error"] = f"{type(err).__name__}: {err}"
            fallback = entry.get("fallback_onnx")
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
                if entry.get("optional") and onnx_path.exists():
                    onnx_path.unlink()
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
