//! Boot splash — drawn with the same framebuf path as the game (no pre-packed buffer).

use crate::framebuf;
use crate::ssd1306::{FRAME_LEN, HEIGHT, WIDTH};

pub const SPLASH_MS: u32 = 3000;

/// 5×7 capitals used on the splash (rows, MSB = leftmost pixel).
const GLYPH_W: usize = 5;
const GLYPH_H: usize = 7;

fn glyph(ch: u8) -> Option<&'static [u8; GLYPH_H]> {
    match ch {
        b'A' => Some(&[0b00100, 0b01010, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        b'D' => Some(&[0b11110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b11110]),
        b'E' => Some(&[0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111]),
        b'H' => Some(&[0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001]),
        b'Q' => Some(&[0b01110, 0b10001, 0b10001, 0b10001, 0b10101, 0b10010, 0b01101]),
        b'R' => Some(&[0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001]),
        b'S' => Some(&[0b01111, 0b10000, 0b10000, 0b01110, 0b00001, 0b00001, 0b11110]),
        b'U' => Some(&[0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110]),
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

/// Paint the SQUARE DASH splash into `frame` (72×40).
pub fn draw(frame: &mut [u8; FRAME_LEN]) {
    framebuf::clear(frame);

    // Outer border
    for x in 0..WIDTH as i32 {
        framebuf::set_pixel(frame, x, 0, true);
        framebuf::set_pixel(frame, x, HEIGHT as i32 - 1, true);
    }
    for y in 0..HEIGHT as i32 {
        framebuf::set_pixel(frame, 0, y, true);
        framebuf::set_pixel(frame, WIDTH as i32 - 1, y, true);
    }

    // Hero cube + face + speed lines
    const CX: i32 = 6;
    const CY: i32 = 11;
    framebuf::fill_rect(frame, CX, CY, 10, 10);
    // eyes
    framebuf::set_pixel(frame, CX + 2, CY + 3, false);
    framebuf::set_pixel(frame, CX + 3, CY + 3, false);
    framebuf::set_pixel(frame, CX + 2, CY + 4, false);
    framebuf::set_pixel(frame, CX + 3, CY + 4, false);
    framebuf::set_pixel(frame, CX + 6, CY + 3, false);
    framebuf::set_pixel(frame, CX + 7, CY + 3, false);
    framebuf::set_pixel(frame, CX + 6, CY + 4, false);
    framebuf::set_pixel(frame, CX + 7, CY + 4, false);
    // smile
    framebuf::set_pixel(frame, CX + 3, CY + 7, false);
    framebuf::set_pixel(frame, CX + 4, CY + 8, false);
    framebuf::set_pixel(frame, CX + 5, CY + 8, false);
    framebuf::set_pixel(frame, CX + 6, CY + 7, false);
    // speed dashes
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

    // Clean ground line + two spikes + one block (no noisy hatch)
    const GROUND: i32 = 34;
    for x in 1..WIDTH as i32 - 1 {
        // gentle bump
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

    // spikes at 18 and 50
    for &(sx, base) in &[(18i32, 33i32), (50, 32)] {
        for row in 0..5 {
            let half = row / 2;
            for dx in -half..=half {
                framebuf::set_pixel(frame, sx + dx, base - 5 + row, true);
            }
        }
    }

    // block
    framebuf::fill_rect(frame, 58, 28, 5, 6);
}
