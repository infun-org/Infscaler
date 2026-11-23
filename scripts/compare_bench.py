#!/usr/bin/env python3
"""Side-by-side center crops: Lanczos upscale of the original vs each model."""

from __future__ import annotations

import csv
from pathlib import Path

from PIL import Image, ImageDraw

ROOT = Path(__file__).resolve().parents[1]
BENCH = ROOT / "samples" / "out" / "bench"
REAL = ROOT / "samples" / "real"
OUT = BENCH / "compare"
CROP = 220


def find_source(stem: str) -> Path | None:
    direct = REAL / stem
    if direct.is_file():
        return direct
    for ext in (".jpg", ".jpeg", ".png", ".webp"):
        path = REAL / f"{stem}{ext}"
        if path.is_file():
            return path
    return None


def lanczos_to(image: Image.Image, size: tuple[int, int]) -> Image.Image:
    if image.size == size:
        return image.convert("RGB")
    return image.convert("RGB").resize(size, Image.Resampling.LANCZOS)


def center_crop(image: Image.Image, edge: int) -> Image.Image:
    w, h = image.size
    edge = min(edge, w, h)
    x = max(0, (w - edge) // 2)
    y = max(0, (h - edge) // 2)
    return image.crop((x, y, x + edge, y + edge))


def label(image: Image.Image, text: str) -> Image.Image:
    bar = 22
    canvas = Image.new("RGB", (image.width, image.height + bar), (18, 18, 20))
    canvas.paste(image, (0, bar))
    draw = ImageDraw.Draw(canvas)
    draw.text((6, 4), text, fill=(230, 230, 230))
    return canvas


def stack(left: Image.Image, right: Image.Image) -> Image.Image:
    gap = 4
    w = left.width + gap + right.width
    h = max(left.height, right.height)
    canvas = Image.new("RGB", (w, h), (8, 8, 8))
    canvas.paste(left, (0, 0))
    canvas.paste(right, (left.width + gap, 0))
    return canvas


def main() -> None:
    csv_path = BENCH / "results.csv"
    if not csv_path.is_file():
        raise SystemExit(f"missing {csv_path}")
    OUT.mkdir(parents=True, exist_ok=True)
    rows = list(csv.DictReader(csv_path.open()))
    made = 0
    for row in rows:
        if row.get("status") != "ok":
            continue
        if row.get("keep_size") not in {"0", "false", "no"}:
            continue
        src = find_source(row["image"])
        dst = Path(row["output"])
        if src is None or not dst.is_file():
            continue
        original = Image.open(src)
        model = Image.open(dst).convert("RGB")
        base = lanczos_to(original, model.size)
        pair = stack(
            label(center_crop(base, CROP), "Lanczos"),
            label(center_crop(model, CROP), row["model"][:42]),
        )
        out = OUT / f"{src.stem}__{row['model']}.png"
        pair.save(out)
        made += 1
        print(out.relative_to(ROOT))
    print(f"{made} comparisons")


if __name__ == "__main__":
    main()
