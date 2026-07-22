//! MONO_VLSB framebuffer helpers (72×40).

use crate::font::{self, ADVANCE, DIGIT_H, DIGIT_W};
use crate::ssd1306::{FRAME_LEN, WIDTH};

pub fn clear(frame: &mut [u8; FRAME_LEN]) {
    frame.fill(0);
}

pub fn set_pixel(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, on: bool) {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= crate::ssd1306::HEIGHT as i32 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    let page = y / 8;
    let bit = y % 8;
    let idx = page * WIDTH + x;
    if on {
        frame[idx] |= 1 << bit;
    } else {
        frame[idx] &= !(1 << bit);
    }
}

pub fn xor_pixel(frame: &mut [u8; FRAME_LEN], x: i32, y: i32) {
    if x < 0 || y < 0 || x >= WIDTH as i32 || y >= crate::ssd1306::HEIGHT as i32 {
        return;
    }
    let x = x as usize;
    let y = y as usize;
    let page = y / 8;
    let bit = y % 8;
    let idx = page * WIDTH + x;
    frame[idx] ^= 1 << bit;
}

pub fn fill_rect(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, w: i32, h: i32) {
    for py in y..y + h {
        for px in x..x + w {
            set_pixel(frame, px, py, true);
        }
    }
}

pub fn clear_rect(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, w: i32, h: i32) {
    for py in y..y + h {
        for px in x..x + w {
            set_pixel(frame, px, py, false);
        }
    }
}

pub fn stroke_rect(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, w: i32, h: i32) {
    if w <= 0 || h <= 0 {
        return;
    }
    for px in x..x + w {
        set_pixel(frame, px, y, true);
        set_pixel(frame, px, y + h - 1, true);
    }
    for py in y..y + h {
        set_pixel(frame, x, py, true);
        set_pixel(frame, x + w - 1, py, true);
    }
}

pub fn fill_spike_up(frame: &mut [u8; FRAME_LEN], base_x: i32, base_y: i32, w: i32, h: i32) {
    // Apex at (base_x + w/2, base_y - h + 1), base from base_x..base_x+w-1 at base_y
    for row in 0..h {
        let t = row; // 0 at apex
        let half = ((t + 1) * w) / (2 * h);
        let cx = base_x + w / 2;
        let y = base_y - h + 1 + row;
        for x in (cx - half)..=(cx + half) {
            set_pixel(frame, x, y, true);
        }
    }
}

pub fn fill_spike_down(frame: &mut [u8; FRAME_LEN], base_x: i32, base_y: i32, w: i32, h: i32) {
    // Base at base_y (ceiling), apex at base_y + h - 1
    for row in 0..h {
        let t = h - 1 - row; // 0 at apex
        let half = ((t + 1) * w) / (2 * h);
        let cx = base_x + w / 2;
        let y = base_y + row;
        for x in (cx - half)..=(cx + half) {
            set_pixel(frame, x, y, true);
        }
    }
}

fn draw_digit(frame: &mut [u8; FRAME_LEN], x: i32, y: i32, d: u8) {
    let Some(rows) = font::digit_rows(d) else {
        return;
    };
    for (row_y, row) in rows.iter().enumerate() {
        for col in 0..DIGIT_W {
            if row & (1 << (DIGIT_W - 1 - col)) != 0 {
                set_pixel(frame, x + col as i32, y + row_y as i32, true);
            }
        }
    }
}

pub fn draw_score(frame: &mut [u8; FRAME_LEN], mut right_x: i32, y: i32, mut score: u32) {
    let mut digits = 1u32;
    let mut n = score;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    let w = (digits as i32 - 1) * ADVANCE as i32 + DIGIT_W as i32;
    let left = right_x - (DIGIT_W as i32 - 1) - (digits as i32 - 1) * ADVANCE as i32;
    // Black backing (+1px pad beyond glyph bounds) so digits stay readable on fill.
    clear_rect(frame, left - 2, y - 1, w + 4, DIGIT_H as i32 + 2);

    loop {
        let d = (score % 10) as u8;
        draw_digit(frame, right_x - (DIGIT_W as i32 - 1), y, d);
        score /= 10;
        if score == 0 {
            break;
        }
        right_x -= ADVANCE as i32;
    }
}
