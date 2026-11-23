#!/usr/bin/env python3
"""8-panel grid matching samples/out/bench2/compare_<image>.jpg.

Each panel is a center 400×400 crop of the 4× output (100×100 of the
original). Order: Lanczos, then the seven bench models, two rows of four.
"""

from __future__ import annotations

import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
MODELS = [
    "realesrgan-x4plus",
    "4xNomos8kSC",
    "ultrasharp-4x",
    "spanx4_ch48",
    "4xNomos2_otf_esrgan",
    "4xRealWebPhoto_v4_dat2",
    "4xNomos2_hq_drct-l",
]
CROP = 400
COORDS = {
    "portrait-blur": (150, 40),
    "dog-jpeg": (150, 60),
    "child": (150, 80),
    "cat": (150, 50),
}
LABEL_H = 25
GUTTER = 10
PAD_BOTTOM = 5


def crop_at(image: Image.Image, stem: str) -> Image.Image:
    w, h = image.size
    x0, y0 = COORDS[stem]
    x = min(x0 * 4, max(0, w - CROP))
    y = min(y0 * 4, max(0, h - CROP))
    return image.crop((x, y, min(x + CROP, w), min(y + CROP, h)))


def panel(image: Image.Image, text: str, font: ImageFont.ImageFont, stem: str) -> Image.Image:
    crop = crop_at(image.convert("RGB"), stem)
    canvas = Image.new("RGB", (crop.width, LABEL_H + crop.height), (255, 255, 255))
    canvas.paste(crop, (0, LABEL_H))
    draw = ImageDraw.Draw(canvas)
    draw.text((4, 6), text, fill=(20, 20, 20), font=font)
    return canvas


def broken_panel(text: str, font: ImageFont.ImageFont) -> Image.Image:
    canvas = Image.new("RGB", (CROP, LABEL_H + CROP), (238, 238, 238))
    draw = ImageDraw.Draw(canvas)
    draw.text((4, 6), text, fill=(20, 20, 20), font=font)
    draw.multiline_text((18, LABEL_H + 140), "BROKEN\nDisabled: tile grid\nartifacts", fill=(145, 30, 30), font=font, spacing=8)
    return canvas


def grid_for(bench: Path, stem: str, font: ImageFont.ImageFont) -> Image.Image:
    source = Image.open(ROOT / "samples" / "real" / f"{stem}.jpg")
    first = Image.open(bench / f"{stem}__{MODELS[0]}.png")
    lanczos = source.convert("RGB").resize(first.size, Image.Resampling.LANCZOS)
    tiles = [panel(lanczos, "original (lanczos)", font, stem)]
    for name in MODELS:
        if name == "4xNomos2_hq_drct-l":
            tiles.append(broken_panel("4xNomos2_hq_drct-l", font))
        else:
            tiles.append(panel(Image.open(bench / f"{stem}__{name}.png"), name, font, stem))
    width = 4 * CROP + 4 * GUTTER
    height = 2 * (LABEL_H + CROP) + PAD_BOTTOM
    canvas = Image.new("RGB", (width, height), (255, 255, 255))
    for index, tile in enumerate(tiles):
        col, row = index % 4, index // 4
        canvas.paste(tile, (col * (CROP + GUTTER), row * (LABEL_H + CROP)))
    return canvas


def main() -> None:
    bench = Path(sys.argv[1] if len(sys.argv) > 1 else ROOT / "samples" / "out" / "bench3")
    font = ImageFont.load_default()
    for stem in ("portrait-blur", "dog-jpeg", "child", "cat"):
        image = grid_for(bench, stem, font)
        dest = bench / f"compare_{stem}.jpg"
        image.save(dest, quality=90)
        print(dest, image.size)


if __name__ == "__main__":
    main()
