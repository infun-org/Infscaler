#!/usr/bin/env python3
"""Download a few freely licensed Commons photos and make low-quality copies.

Writes samples/real/ and samples/real/SOURCES.md.
"""

from __future__ import annotations

import json
import subprocess
import time
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / "samples" / "real"
UA = "InfScaler/0.2 (local photo research; python-urllib)"

# Pinned Commons files so a re-run does not pick a painting from search.
# Licenses are still checked from live metadata before anything is saved.
PINS = [
    ("dog", "File:A Golden Retriever-9 (Barras).JPG"),
    ("cat", "File:Cat playing with a lizard.jpg"),
    ("portrait", "File:Laughing Woman (Imagicity 1162).jpg"),
    ("person", "File:Young men smiling with teeth, studio portrait.jpg"),
    ("child", "File:ChildPlay.jpg"),
    ("horse", "File:Horse December 2014-1.jpg"),
]

SEARCHES = [
    ("dog", "photograph golden retriever outdoor -painting"),
    ("cat", "photograph domestic cat face -painting -drawing"),
    ("portrait", "photograph woman portrait face outdoor -painting -oil -drawing"),
    ("person", "photograph man smiling studio portrait -painting"),
    ("child", "photograph children playing outdoor -painting"),
    ("horse", "photograph horse head -painting -drawing"),
]

REJECT = ("vermeer", "painting", "oil on", "de la haye", "canvas", "drawing")

FREE = (
    "cc0",
    "cc by",
    "cc-by",
    "cc by-sa",
    "cc-by-sa",
    "public domain",
)


def api(params: dict) -> dict:
    params = {"format": "json", "origin": "*", **params}
    url = "https://commons.wikimedia.org/w/api.php?" + urllib.parse.urlencode(params)
    req = urllib.request.Request(url, headers={"User-Agent": UA})
    time.sleep(1.2)
    with urllib.request.urlopen(req, timeout=60) as response:
        return json.load(response)


def license_ok(meta: dict) -> str | None:
    short = (meta.get("LicenseShortName", {}).get("value") or "").strip()
    low = short.lower()
    if "nc" in low or "nd" in low:
        return None
    if any(token in low for token in FREE):
        return short
    return None


def record_from_page(page: dict) -> dict | None:
    info = (page.get("imageinfo") or [None])[0]
    if not info:
        return None
    meta = info.get("extmetadata") or {}
    short = license_ok(meta)
    if not short:
        return None
    title_low = page.get("title", "").lower()
    desc_low = (meta.get("ImageDescription", {}).get("value") or "").lower()
    if any(word in title_low or word in desc_low for word in REJECT):
        return None
    if info.get("mime") not in {"image/jpeg", "image/png"}:
        return None
    width = int(info.get("width") or info.get("thumbwidth") or 0)
    height = int(info.get("height") or info.get("thumbheight") or 0)
    if min(width, height) < 400:
        return None
    title = page.get("title", "")
    filename = title.split(":", 1)[-1]
    url = (
        "https://commons.wikimedia.org/wiki/Special:FilePath/"
        + urllib.parse.quote(filename, safe="")
        + "?width=900"
    )
    artist = (meta.get("Artist", {}).get("value") or "").replace("\n", " ")
    credit = (meta.get("Credit", {}).get("value") or "")
    desc = (meta.get("ImageDescription", {}).get("value") or title)
    return {
        "title": title,
        "url": url,
        "page": "https://commons.wikimedia.org/wiki/" + urllib.parse.quote(title.replace(" ", "_")),
        "license": short,
        "artist": strip_html(artist)[:240],
        "description": strip_html(desc)[:240],
        "credit": strip_html(credit)[:180],
    }


def pinned() -> dict[str, dict]:
    data = api(
        {
            "action": "query",
            "titles": "|".join(title for _, title in PINS),
            "prop": "imageinfo",
            "iiprop": "url|size|mime|extmetadata",
        }
    )
    pages = {page.get("title"): page for page in data.get("query", {}).get("pages", {}).values()}
    found = {}
    for stem, title in PINS:
        page = pages.get(title)
        if not page:
            print(f"pin missing {title}")
            continue
        record = record_from_page(page)
        if record:
            found[stem] = record
        else:
            print(f"pin rejected {title}")
    return found


def search_one(query: str) -> dict | None:
    data = api(
        {
            "action": "query",
            "generator": "search",
            "gsrsearch": f"filetype:bitmap {query}",
            "gsrnamespace": "6",
            "gsrlimit": "12",
            "prop": "imageinfo",
            "iiprop": "url|size|mime|extmetadata",
            "iiurlwidth": "1600",
        }
    )
    pages = data.get("query", {}).get("pages", {})
    ranked = sorted(pages.values(), key=lambda page: page.get("index", 99))
    for page in ranked:
        record = record_from_page(page)
        if record:
            return record
    return None


def strip_html(text: str) -> str:
    out = []
    skip = False
    for ch in text:
        if ch == "<":
            skip = True
            continue
        if ch == ">":
            skip = False
            continue
        if not skip:
            out.append(ch)
    return " ".join("".join(out).split())


def download(url: str, dest: Path) -> None:
    for attempt in range(5):
        time.sleep(2.5 + attempt * 4)
        result = subprocess.run(
            ["curl", "-fsSL", "-A", UA, "-L", "--retry", "2", "--max-time", "90", "-o", str(dest), url],
            check=False,
        )
        if result.returncode == 0 and dest.is_file() and dest.stat().st_size > 2000:
            return
        print(f"retry {attempt + 1} for {url}")
    raise SystemExit(f"could not download {url}")


def magick_resize(src: Path, dest: Path, extra: list[str]) -> None:
    subprocess.check_call(["magick", str(src), *extra, str(dest)])


def main() -> None:
    OUT.mkdir(parents=True, exist_ok=True)
    picked = []
    used_titles = set()
    pins = pinned()
    for stem, query in SEARCHES:
        found = pins.get(stem) or search_one(query)
        if not found or found["title"] in used_titles:
            print(f"skip {stem}: nothing free")
            continue
        used_titles.add(found["title"])
        raw = OUT / f"_{stem}.img"
        download(found["url"], raw)
        dest = OUT / f"{stem}.jpg"
        magick_resize(raw, dest, ["-auto-orient", "-resize", "400x400>", "-strip", "-quality", "90"])
        raw.unlink(missing_ok=True)
        found["file"] = dest.name
        found["kind"] = "resized, long edge at most 400 px"
        picked.append(found)
        print(f"saved {dest.name}: {found['title']} ({found['license']})")

    # Two degraded copies so the bench includes soft / recompressed sources.
    degraded = []
    by_file = {item["file"]: item for item in picked}
    if "portrait.jpg" in by_file:
        src = OUT / "portrait.jpg"
        dest = OUT / "portrait-blur.jpg"
        magick_resize(src, dest, ["-blur", "0x1.1", "-quality", "40"])
        item = dict(by_file["portrait.jpg"])
        item["file"] = dest.name
        item["kind"] = "same photo, Gaussian blur 0x1.1, JPEG quality 40"
        degraded.append(item)
    if "dog.jpg" in by_file:
        src = OUT / "dog.jpg"
        dest = OUT / "dog-jpeg.jpg"
        magick_resize(src, dest, ["-resize", "50%", "-resize", "200%", "-quality", "35"])
        item = dict(by_file["dog.jpg"])
        item["file"] = dest.name
        item["kind"] = "same photo, downscaled to half and back, JPEG quality 35"
        degraded.append(item)

    lines = [
        "# Real photo samples",
        "",
        "Freely licensed photographs from Wikimedia Commons, resized so a CPU",
        "benchmark on this machine stays tractable. Two files are further degraded",
        "to stand in for a soft or heavily compressed source.",
        "",
        "| File | License | Source | What we did |",
        "| --- | --- | --- | --- |",
    ]
    for item in picked + degraded:
        lines.append(
            f"| `{item['file']}` | {item['license']} | [{item['title']}]({item['page']}) — {item['artist']} | {item['kind']} |"
        )
    lines += [
        "",
        "Descriptions are shortened from the Commons metadata. The page link is the",
        "authoritative source for the license and attribution.",
        "",
    ]
    (OUT / "SOURCES.md").write_text("\n".join(lines))
    print(f"wrote {len(picked)} photos and {len(degraded)} degraded copies")


if __name__ == "__main__":
    main()
