#!/usr/bin/env python3
from pathlib import Path

from PIL import Image, ImageFilter

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "fixtures/sources/trees/iso_pines.png"
PNG_OUT = ROOT / "crates/sim-render/assets/tree_cards.png"
RGBA_OUT = ROOT / "crates/sim-render/assets/tree_cards.rgba"

CELL_WIDTH = 128
CELL_HEIGHT = 256
MARGIN = 3
OUTLINE_PX = 0

CROPS = [
    (0, 8, 96, 254),
    (99, 19, 189, 251),
    (192, 52, 289, 254),
    (289, 52, 384, 254),
    (269, 263, 398, 510),
]


def fade_sprite(sprite: Image.Image) -> Image.Image:
    pixels = sprite.load()
    for y in range(sprite.height):
        for x in range(sprite.width):
            red, green, blue, alpha = pixels[x, y]
            if alpha == 0 or max(red, green, blue) <= 12:
                continue
            r = (red / 255.0) ** 0.70
            g = (green / 255.0) ** 0.70
            b = (blue / 255.0) ** 0.70
            luminance = 0.2126 * r + 0.7152 * g + 0.0722 * b
            r = r * 0.84 + luminance * 0.16
            g = g * 0.84 + luminance * 0.16
            b = b * 0.84 + luminance * 0.16
            r = r * 0.93 + 0.50 * 0.07 + 0.03
            g = g * 0.93 + 0.60 * 0.07 + 0.03
            b = b * 0.93 + 0.44 * 0.07 + 0.03
            pixels[x, y] = (
                round(r * 255),
                round(g * 255),
                round(b * 255),
                alpha,
            )
    return sprite


def main() -> None:
    source = Image.open(SOURCE).convert("RGBA")
    atlas = Image.new("RGBA", (CELL_WIDTH * len(CROPS), CELL_HEIGHT), (0, 0, 0, 0))
    for index, box in enumerate(CROPS):
        crop = source.crop(box)
        alpha = crop.getchannel("A").point(lambda value: 255 if value > 16 else 0)
        bounds = alpha.getbbox()
        if bounds:
            crop = crop.crop(bounds)
        scale = min(
            (CELL_WIDTH - MARGIN * 2) / crop.width,
            (CELL_HEIGHT - MARGIN * 2) / crop.height,
        )
        width = max(1, round(crop.width * scale))
        height = max(1, round(crop.height * scale))
        sprite = crop.resize((width, height), Image.LANCZOS)
        alpha = sprite.getchannel("A").point(lambda value: 255 if value > 24 else 0)
        dilated = (
            alpha.filter(ImageFilter.MaxFilter(OUTLINE_PX * 2 + 1))
            if OUTLINE_PX > 0
            else alpha
        )
        outlined = Image.new("RGBA", sprite.size, (8, 10, 8, 0))
        outlined.putalpha(dilated)
        outlined.alpha_composite(sprite)
        outlined = fade_sprite(outlined)
        x = index * CELL_WIDTH + (CELL_WIDTH - width) // 2
        y = (CELL_HEIGHT - height) // 2
        atlas.alpha_composite(outlined, (x, y))

    PNG_OUT.parent.mkdir(parents=True, exist_ok=True)
    atlas.save(PNG_OUT)
    RGBA_OUT.write_bytes(atlas.tobytes())
    print(f"wrote {PNG_OUT} and {RGBA_OUT} ({atlas.width}x{atlas.height})")


if __name__ == "__main__":
    main()
