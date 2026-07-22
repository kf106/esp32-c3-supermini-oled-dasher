//! Compact 3×5 digit font for score / HUD on 72×40.

pub const DIGIT_W: usize = 3;
pub const DIGIT_H: usize = 5;
pub const ADVANCE: usize = 4;

/// Each digit is 5 rows; bits 2..0 are left→right pixels (MSB = leftmost).
const DIGITS: [[u8; DIGIT_H]; 10] = [
    [0b111, 0b101, 0b101, 0b101, 0b111], // 0
    [0b010, 0b110, 0b010, 0b010, 0b111], // 1
    [0b111, 0b001, 0b111, 0b100, 0b111], // 2
    [0b111, 0b001, 0b111, 0b001, 0b111], // 3
    [0b101, 0b101, 0b111, 0b001, 0b001], // 4
    [0b111, 0b100, 0b111, 0b001, 0b111], // 5
    [0b111, 0b100, 0b111, 0b101, 0b111], // 6
    [0b111, 0b001, 0b001, 0b001, 0b001], // 7
    [0b111, 0b101, 0b111, 0b101, 0b111], // 8
    [0b111, 0b101, 0b111, 0b001, 0b111], // 9
];

pub fn digit_rows(d: u8) -> Option<&'static [u8; DIGIT_H]> {
    if d < 10 {
        Some(&DIGITS[d as usize])
    } else {
        None
    }
}
