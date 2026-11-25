#!/usr/bin/env python3
"""Builds the icon library of Elementos → Iconos (spec 103) from an unpacked npm
`@tabler/icons` package: assets/icons/tabler/tabler.json.gz and its LICENSE.

    npm pack @tabler/icons@3.53.0 && tar xzf tabler-icons-3.53.0.tgz
    python3 tools/gen_tabler_icons.py package

Brand logos are left out: they are other companies' trademarks.
"""
import gzip
import json
import re
import shutil
import sys
from pathlib import Path

# The categories in the order the panel shows them; the rest follow by name.
ORDER = [
    "Arrows", "Shapes", "Symbols", "Communication", "E-commerce", "Document", "Media",
    "Photography", "Design", "Devices", "Computers", "Map", "Buildings", "Vehicles",
    "Weather", "Nature", "Animals", "Food", "Health", "Sport", "Games", "Mood",
    "Gestures", "Charts", "Currencies", "Math", "Numbers", "Letters", "Text", "Badges",
    "Zodiac", "Gender", "Laundry", "Electrical", "Database", "Development",
    "Version control", "Logic", "Extensions", "System",
]
SKIP = {"Brand"}


def body(svg: str) -> str:
    """The drawing of a Tabler SVG without its wrapper and its invisible frame."""
    inner = re.sub(r"^.*?<svg[^>]*>", "", svg, flags=re.S)
    inner = re.sub(r"</svg>\s*$", "", inner, flags=re.S)
    inner = re.sub(r'<path stroke="none" d="M0 0h24v24H0z" fill="none"\s*/>', "", inner)
    return re.sub(r"\s+", " ", inner).strip()


def main(package: Path) -> None:
    meta = json.loads((package / "icons.json").read_text())
    version = json.loads((package / "package.json").read_text())["version"]
    names = sorted({m.get("category") or "System" for m in meta.values()} - SKIP)
    categories = [c for c in ORDER if c in names] + [c for c in names if c not in ORDER]
    index = {c: i for i, c in enumerate(categories)}
    icons = []
    for name in sorted(meta):
        m = meta[name]
        category = m.get("category") or "System"
        if category in SKIP:
            continue
        outline = package / "icons" / "outline" / f"{name}.svg"
        filled = package / "icons" / "filled" / f"{name}.svg"
        icons.append([
            name,
            index[category],
            " ".join(str(t) for t in m.get("tags", [])),
            body(outline.read_text()) if outline.exists() else "",
            body(filled.read_text()) if filled.exists() else "",
        ])
    out = Path(__file__).resolve().parent.parent / "assets" / "icons" / "tabler"
    out.mkdir(parents=True, exist_ok=True)
    data = json.dumps({"version": version, "categories": categories, "icons": icons},
                      separators=(",", ":"), ensure_ascii=False).encode()
    (out / "tabler.json.gz").write_bytes(gzip.compress(data, compresslevel=9, mtime=0))
    shutil.copy(package / "LICENSE", out / "LICENSE")
    print(f"{len(icons)} icons in {len(categories)} categories, "
          f"{len(data) // 1024} KB, {(out / 'tabler.json.gz').stat().st_size // 1024} KB gzipped")


if __name__ == "__main__":
    main(Path(sys.argv[1]))
