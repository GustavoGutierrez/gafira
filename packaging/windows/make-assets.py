#!/usr/bin/env python3
"""The pictures of the Windows installer (spec 090), from the brand icons.

Modern UI 2 wants 24-bit BMP files: 164 x 314 for the welcome page and
150 x 57 for the header. They are committed; only a change of the brand needs
this again:

    python3 packaging/windows/make-assets.py

Requires Pillow.
"""

from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[2]
ICON = ROOT / "assets/app-icon/hicolor/512x512/apps/gafira.png"
OUT = Path(__file__).resolve().parent / "assets"
FONT = ROOT / "assets/fonts/library/manrope"
INK = (24, 36, 59)
MINT = (62, 191, 168)
WHITE = (255, 255, 255)
SOFT = (170, 182, 203)


def font(size):
    file = sorted(FONT.glob("*.ttf"))[0]
    return ImageFont.truetype(str(file), size)


def icon(side):
    return Image.open(ICON).convert("RGBA").resize((side, side), Image.LANCZOS)


def welcome():
    page = Image.new("RGB", (164, 314), INK)
    draw = ImageDraw.Draw(page)
    mark = icon(96)
    page.paste(mark, (34, 62), mark)
    draw.text((82, 190), "Gafira", font=font(26), fill=WHITE, anchor="mm")
    draw.rectangle((62, 212, 102, 215), fill=MINT)
    draw.text((82, 236), "Haz visibles tus ideas.", font=font(11), fill=SOFT, anchor="mm")
    page.save(OUT / "welcome.bmp")


def header():
    page = Image.new("RGB", (150, 57), WHITE)
    draw = ImageDraw.Draw(page)
    mark = icon(40)
    page.paste(mark, (102, 8), mark)
    draw.text((94, 28), "Gafira", font=font(17), fill=INK, anchor="rm")
    page.save(OUT / "header.bmp")


if __name__ == "__main__":
    OUT.mkdir(exist_ok=True)
    welcome()
    header()
