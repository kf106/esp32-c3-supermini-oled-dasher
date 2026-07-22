//! Boot splash + all-clear screen — same framebuf path as the game.

use crate::framebuf;
use crate::ssd1306::{FRAME_LEN, HEIGHT, WIDTH};

pub const SPLASH_MS: u32 = 3000;

/// 5×7 capitals used on splash / complete screens (rows, MSB = leftmost pixel).
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;

fn glyph(ch: u8) -> Option<&'static [u8; GLYPH_H]> {
    match ch {
        b'A' => Some(&[0b00100, 0b01010, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        b'C' => Some(&[0b01110, 0b10001, 0b10000, 0b10000, 0b10000, 0b10001, 0b01110]),
        b'D' => Some(&[0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        b'E' => Some(&[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        b'H' => Some(&[0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        b'L' => Some(&[0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111]),
        b'M' => Some(&[0b10001, 0b11011, 0b10101, 0b10001, 0b10001, 0b10001, 0b10001]),
        b'O' => Some(&[0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        b'P' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10000, 0b10000, 0b10000]),
        b'Q' => Some(&[0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        b'R' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        b'S' => Some(&[0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        b'T' => Some(&[0b11111, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00100]),
        b'U' => Some(&[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
        b'!' => Some(&[0b00100, 0b00100, 0b00100, 0b00100, 0b00100, 0b00000, 0b00100]),
        b' ' => Some(&[0, 0, 0, 0, 0, 0, 0]),
        _ => None,
    }
}

fn draw_char(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, ch: u8) {
    let Some(rows) = glyph(ch) else {
        return;
    };
    for (row_i, row) in rows.iter().enumerate() {
        for col in 0..GLYPH_W {
            if row & (1 << (GLYPH_W - 1 - col)) != 0 {
                framebuf::set_pixel(frame, x + col as i32, y + row_i as i32, true);
            }
        }
    }
}

fn draw_text(frame: &mut [u8; FRAME_LEN], mut x: i32, y: i32, text: &[u8]) {
    for &ch in text {
        draw_char(frame, x, y, ch);
        x += GLYPH_W as i32 + 1;
    }
}

fn text_width(text: &[u8]) -> i32 {
    if text.is_empty() {
        return 0;
    }
    text.len() as i32 * (GLYPH_W as i32 + 1) - 1
}

fn border(frame: &mut [u8; FRAME_LEN]) {
    for x in 0..WIDTH as i32 {
        framebuf::set_pixel(frame, x, 0, true);
        framebuf::set_pixel(frame, x, HEIGHT as i32 - 1, true);
    }
    for y in 0..HEIGHT as i32 {
        framebuf::set_pixel(frame, 0, y, true);
        framebuf::set_pixel(frame, WIDTH as i32 - 1, y, true);
    }
}

fn draw_hero_cube(frame: &mut [u8; FRAME_LEN], cx: i32, cy: i32) {
    framebuf::fill_rect(frame, cx, cy, 10, 10);
    // eyes
    framebuf::set_pixel(frame, cx + 2, cy + 3, false);
    framebuf::set_pixel(frame, cx + 3, cy + 3, false);
    framebuf::set_pixel(frame, cx + 2, cy + 4, false);
    framebuf::set_pixel(frame, cx + 3, cy + 4, false);
    framebuf::set_pixel(frame, cx + 6, cy + 3, false);
    framebuf::set_pixel(frame, cx + 7, cy + 3, false);
    framebuf::set_pixel(frame, cx + 6, cy + 4, false);
    framebuf::set_pixel(frame, cx + 7, cy + 4, false);
    // smile
    framebuf::set_pixel(frame, cx + 3, cy + 7, false);
    framebuf::set_pixel(frame, cx + 4, cy + 8, false);
    framebuf::set_pixel(frame, cx + 5, cy + 8, false);
    framebuf::set_pixel(frame, cx + 6, cy + 7, false);
}

/// Paint the SQUARE DASH splash into `frame` (72×40).
pub fn draw(frame: &mut [u8; FRAME_LEN]) {
    framebuf::clear(frame);
    border(frame);

    // Hero cube + face + speed lines
    const CX: i32 = 6;
    const CY: i32 = 11;
    draw_hero_cube(frame, CX, CY);
    for (i, len) in [3i32, 5, 4].iter().enumerate() {
        let y = CY + 3 + i as i32 * 2;
        for t in 0..*len {
            framebuf::set_pixel(frame, CX - 2 - t, y, true);
        }
    }

    // Title box
    const BX0: i32 = 20;
    const BY0: i32 = 6;
    const BX1: i32 = 66;
    const BY1: i32 = 28;
    for x in BX0..=BX1 {
        framebuf::set_pixel(frame, x, BY0, true);
        framebuf::set_pixel(frame, x, BY1, true);
    }
    for y in BY0..=BY1 {
        framebuf::set_pixel(frame, BX0, y, true);
        framebuf::set_pixel(frame, BX1, y, true);
    }

    draw_text(frame, 24, 9, b"SQUARE");
    draw_text(frame, 30, 18, b"DASH");
    for x in 29..54 {
        framebuf::set_pixel(frame, x, 26, true);
    }

    // Clean ground line + two spikes + one block
    const GROUND: i32 = 34;
    for x in 1..WIDTH as i32 - 1 {
        let bump = match x {
            10..=20 => -1,
            21..=28 => -2,
            29..=36 => -1,
            45..=55 => -2,
            56..=62 => -1,
            _ => 0,
        };
        let gy = GROUND + bump;
        framebuf::set_pixel(frame, x, gy, true);
    }

    for &(sx, base) in &[(18i32, 33i32), (50, 32)] {
        for row in 0..5 {
            let half = row / 2;
            for dx in -half..=half {
                framebuf::set_pixel(frame, sx + dx, base - 5 + row, true);
            }
        }
    }

    framebuf::fill_rect(frame, 58, 28, 5, 6);
}

/// All 16 levels cleared — `SQUARE DASH COMPLETE!` (72×40).
pub fn draw_complete(frame: &mut [u8; FRAME_LEN]) {
    framebuf::clear(frame);
    border(frame);

    // Celebrating cube, centered above the text
    draw_hero_cube(frame, 31, 3);

    // Confetti dots
    for &(x, y) in &[
        (8i32, 5i32),
        (14, 8),
        (20, 4),
        (52, 6),
        (58, 4),
        (64, 8),
        (10, 14),
        (62, 14),
    ] {
        framebuf::set_pixel(frame, x, y, true);
    }

    let line1 = b"SQUARE DASH";
    let line2 = b"COMPLETE!";
    let x1 = (WIDTH as i32 - text_width(line1)) / 2;
    let x2 = (WIDTH as i32 - text_width(line2)) / 2;
    draw_text(frame, x1, 16, line1);
    draw_text(frame, x2, 27, line2);
}
