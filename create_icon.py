#!/usr/bin/env python3
"""Generate app icon for Rachael's Transcriber.

Produces:
  assets/icon.png           — 1024×1024 source
  assets/icon.iconset/      — iconset dir for macOS .icns
  assets/icon.icns          — macOS icon
  assets/icon.ico           — Windows icon (multi-res)

Requires: Pillow
"""

import shutil
import subprocess
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

HERE = Path(__file__).parent.resolve()
ASSETS = HERE / "assets"
SRC_SIZE = 1024
BG_COLOR = "#007AFF"
TEXT_COLOR = "#FFFFFF"

ICONSET_SIZES = [
    (16, 1),
    (16, 2),  # 16@2 → 32
    (32, 1),
    (32, 2),  # 32@2 → 64
    (128, 1),
    (128, 2),  # 128@2 → 256
    (256, 1),
    (256, 2),  # 256@2 → 512
    (512, 1),
    (512, 2),  # 512@2 → 1024
]

ICO_SIZES = [16, 32, 48, 64, 256]


def find_font():
    candidates = [
        "/System/Library/Fonts/Supplemental/Arial Bold.ttf",
        "/System/Library/Fonts/Helvetica.ttc",
        "/Library/Fonts/Arial.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
    ]
    for p in candidates:
        if Path(p).exists():
            return p
    return None


def _load_font(font_path, size):
    if font_path is None:
        return ImageFont.load_default()
    try:
        if font_path.endswith(".ttc"):
            return ImageFont.truetype(font_path, size, index=1)
        return ImageFont.truetype(font_path, size)
    except Exception:
        return ImageFont.load_default()


def draw_icon(size, font):
    """Draw the icon at a given size. Returns an RGBA Image."""
    img = Image.new("RGBA", (size, size), BG_COLOR)
    draw = ImageDraw.Draw(img)

    text = "RT"
    font_size = size * 450 // 1024
    fnt = _load_font(font, font_size)

    bbox = draw.textbbox((0, 0), text, font=fnt)
    tw = bbox[2] - bbox[0]
    th = bbox[3] - bbox[1]
    tx = (size - tw) // 2 - bbox[0]
    ty = (size - th) // 2 - bbox[1]
    draw.text((tx, ty), text, font=fnt, fill=TEXT_COLOR)

    return img


def generate_png(font):
    img = draw_icon(SRC_SIZE, font)
    out = ASSETS / "icon.png"
    img.save(out)
    print(f"  Saved {out} ({img.size[0]}×{img.size[1]})")
    return img


def generate_iconset(src_img):
    iconset = ASSETS / "icon.iconset"
    if iconset.is_dir():
        shutil.rmtree(iconset)
    iconset.mkdir(parents=True, exist_ok=True)

    for base_size, scale in ICONSET_SIZES:
        px = base_size * scale
        suffix = f"_{base_size}x{base_size}@2x" if scale == 2 else f"_{base_size}x{base_size}"
        name = f"icon{suffix}.png"
        path = iconset / name
        if px == SRC_SIZE:
            img = src_img.copy()
        else:
            img = src_img.resize((px, px), Image.LANCZOS)
        img.save(path)
        print(f"  Saved {path} ({img.size[0]}×{img.size[1]})")

    # Convert with iconutil
    icns_path = ASSETS / "icon.icns"
    try:
        subprocess.run(
            ["iconutil", "-c", "icns", str(iconset), "-o", str(icns_path)],
            check=True, capture_output=True, text=True,
        )
        print(f"  Saved {icns_path}")
    except FileNotFoundError:
        print("  [SKIP] iconutil not found (macOS only)")
    except subprocess.CalledProcessError as e:
        print(f"  [ERROR] iconutil failed: {e.stderr}")


def generate_ico(src_img):
    ico_path = ASSETS / "icon.ico"
    images = []
    for s in ICO_SIZES:
        if s == SRC_SIZE:
            img = src_img.copy()
        else:
            img = src_img.resize((s, s), Image.LANCZOS)
        images.append(img)
    # ICO expects RGBA
    images[0].save(
        ico_path,
        format="ICO",
        sizes=[(s, s) for s in ICO_SIZES],
        append_images=images[1:],
    )
    print(f"  Saved {ico_path} (multi-res: {ICO_SIZES})")


def main():
    font_path = find_font()
    if font_path:
        print(f"  Font: {font_path}")
    else:
        print("  Font: default (no bold TTF found)")

    ASSETS.mkdir(parents=True, exist_ok=True)

    print("Generating source PNG (1024×1024)...")
    src_img = generate_png(font_path)

    print("\nGenerating .iconset and .icns...")
    generate_iconset(src_img)

    print("\nGenerating .ico...")
    generate_ico(src_img)

    print("\nDone.")
    print(f"\n  Source:  {ASSETS / 'icon.png'}")
    print(f"  macOS:   {ASSETS / 'icon.icns'}")
    print(f"  Windows: {ASSETS / 'icon.ico'}")


if __name__ == "__main__":
    main()
