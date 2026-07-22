#!/usr/bin/env python3
"""Render splash / complete PNGs to match firmware/src/splash.rs exactly.

Simulates the same MONO_VLSB set_pixel / fill_rect / text path used on device.
"""

from __future__ import annotations

import pathlib

from PIL import Image

ROOT = pathlib.Path(__file__).resolve().parents[2]
ASSETS = ROOT / "assets"

WIDTH, HEIGHT = 72, 40

# Must match splash.rs glyph table (MSB = left)
GLYPHS = {
    "A": [0b00100, 0b01010, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    "C": [0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110],
    "D": [0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110],
    "E": [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
    "H": [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
    "L": [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
    "M": [0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001],
    "O": [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    "P": [0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000],
    "Q": [0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101],
    "R": [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
    "S": [0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110],
    "T": [0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100],
    "U": [0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
    "!": [0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100],
    " ": [0, 0, 0, 0, 0, 0, 0],
}
GLYPH_W, GLYPH_H = 5, 7


def new_canvas():
    return [[False] * WIDTH for _ in range(HEIGHT)]


def export(pix, stem: str) -> None:
    ASSETS.mkdir(parents=True, exist_ok=True)
    native = Image.new("L", (WIDTH, HEIGHT), 0)
    n = native.load()
    for y in range(HEIGHT):
        for x in range(WIDTH):
            if pix[y][x]:
                n[x, y] = 255
    native.save(ASSETS / f"{stem}-72x40.png")

    preview = native.resize((WIDTH * 10, HEIGHT * 10), Image.NEAREST)
    rgb = Image.new("RGB", preview.size, (10, 12, 16))
    p = preview.load()
    for y in range(preview.size[1]):
        for x in range(preview.size[0]):
            if p[x, y] > 127:
                rgb.putpixel((x, y), (230, 235, 240))
    rgb.save(ASSETS / f"{stem}.png")
    if stem == "splash":
        rgb.save(ROOT / "splash.png")
    print(f"wrote {ASSETS / f'{stem}.png'} (matches on-device draw)")


def draw_splash(pix) -> None:
    def set_pixel(x: int, y: int, on: bool = True) -> None:
        if 0 <= x < WIDTH and 0 <= y < HEIGHT:
            pix[y][x] = on

    def fill_rect(x: int, y: int, w: int, h: int) -> None:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                set_pixel(xx, yy, True)

    def draw_char(x: int, y: int, ch: str) -> None:
        rows = GLYPHS[ch]
        for row_i, row in enumerate(rows):
            for col in range(GLYPH_W):
                if row & (1 << (GLYPH_W - 1 - col)):
                    set_pixel(x + col, y + row_i, True)

    def draw_text(x: int, y: int, text: str) -> None:
        cx = x
        for ch in text:
            draw_char(cx, y, ch)
            cx += GLYPH_W + 1

    for x in range(WIDTH):
        set_pixel(x, 0)
        set_pixel(x, HEIGHT - 1)
    for y in range(HEIGHT):
        set_pixel(0, y)
        set_pixel(WIDTH - 1, y)

    CX, CY = 6, 11
    fill_rect(CX, CY, 10, 10)
    for ox, oy in [(2, 3), (3, 3), (2, 4), (3, 4), (6, 3), (7, 3), (6, 4), (7, 4)]:
        set_pixel(CX + ox, CY + oy, False)
    for ox, oy in [(3, 7), (4, 8), (5, 8), (6, 7)]:
        set_pixel(CX + ox, CY + oy, False)
    for i, length in enumerate((3, 5, 4)):
        y = CY + 3 + i * 2
        for t in range(length):
            set_pixel(CX - 2 - t, y, True)

    BX0, BY0, BX1, BY1 = 20, 6, 66, 28
    for x in range(BX0, BX1 + 1):
        set_pixel(x, BY0)
        set_pixel(x, BY1)
    for y in range(BY0, BY1 + 1):
        set_pixel(BX0, y)
        set_pixel(BX1, y)

    draw_text(24, 9, "SQUARE")
    draw_text(30, 18, "DASH")
    for x in range(29, 54):
        set_pixel(x, 26)

    GROUND = 34
    for x in range(1, WIDTH - 1):
        if 10 <= x <= 20:
            bump = -1
        elif 21 <= x <= 28:
            bump = -2
        elif 29 <= x <= 36:
            bump = -1
        elif 45 <= x <= 55:
            bump = -2
        elif 56 <= x <= 62:
            bump = -1
        else:
            bump = 0
        set_pixel(x, GROUND + bump)

    for sx, base in ((18, 33), (50, 32)):
        for row in range(5):
            half = row // 2
            for dx in range(-half, half + 1):
                set_pixel(sx + dx, base - 5 + row)

    fill_rect(58, 28, 5, 6)


def draw_complete(pix) -> None:
    def set_pixel(x: int, y: int, on: bool = True) -> None:
        if 0 <= x < WIDTH and 0 <= y < HEIGHT:
            pix[y][x] = on

    def fill_rect(x: int, y: int, w: int, h: int) -> None:
        for yy in range(y, y + h):
            for xx in range(x, x + w):
                set_pixel(xx, yy, True)

    def draw_char(x: int, y: int, ch: str) -> None:
        rows = GLYPHS[ch]
        for row_i, row in enumerate(rows):
            for col in range(GLYPH_W):
                if row & (1 << (GLYPH_W - 1 - col)):
                    set_pixel(x + col, y + row_i, True)

    def draw_text(x: int, y: int, text: str) -> None:
        cx = x
        for ch in text:
            draw_char(cx, y, ch)
            cx += GLYPH_W + 1

    def text_width(text: str) -> int:
        if not text:
            return 0
        return len(text) * (GLYPH_W + 1) - 1

    for x in range(WIDTH):
        set_pixel(x, 0)
        set_pixel(x, HEIGHT - 1)
    for y in range(HEIGHT):
        set_pixel(0, y)
        set_pixel(WIDTH - 1, y)

    CX, CY = 31, 3
    fill_rect(CX, CY, 10, 10)
    for ox, oy in [(2, 3), (3, 3), (2, 4), (3, 4), (6, 3), (7, 3), (6, 4), (7, 4)]:
        set_pixel(CX + ox, CY + oy, False)
    for ox, oy in [(3, 7), (4, 8), (5, 8), (6, 7)]:
        set_pixel(CX + ox, CY + oy, False)

    for x, y in (
        (8, 5),
        (14, 8),
        (20, 4),
        (52, 6),
        (58, 4),
        (64, 8),
        (10, 14),
        (62, 14),
    ):
        set_pixel(x, y)

    line1, line2 = "SQUARE DASH", "COMPLETE!"
    draw_text((WIDTH - text_width(line1)) // 2, 16, line1)
    draw_text((WIDTH - text_width(line2)) // 2, 27, line2)


def main() -> None:
    splash = new_canvas()
    draw_splash(splash)
    export(splash, "splash")

    complete = new_canvas()
    draw_complete(complete)
    export(complete, "complete")


if __name__ == "__main__":
    main()
