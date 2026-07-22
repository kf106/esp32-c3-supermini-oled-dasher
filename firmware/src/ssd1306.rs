//! Minimal SSD1306 driver (72×40, I2C) — MONO_VLSB framebuffer.

use esp_hal::{
    i2c::{I2c, Instance},
    Blocking,
};

const ADDR: u8 = 0x3C;
pub const WIDTH: usize = 72;
pub const HEIGHT: usize = 40;
/// Visible 72px window on the panel's 128-column GDDRAM (see MicroPython driver).
const COL_OFFSET: u8 = 28;
pub const FRAME_LEN: usize = WIDTH * HEIGHT / 8;

pub struct Display<'d, T> {
    i2c: I2c<'d, T, Blocking>,
}

impl<'d, T> Display<'d, T>
where
    T: Instance,
{
    pub fn new(i2c: I2c<'d, T, Blocking>) -> Self {
        let mut d = Self { i2c };
        d.init();
        d
    }

    fn cmd(&mut self, byte: u8) {
        let buf = [0x00, byte];
        let _ = self.i2c.write(ADDR, &buf);
    }

    /// SSD1306 I2C data writes must be prefixed with `0x40` (unlike commands `0x00`).
    fn write_data(&mut self, data: &[u8]) {
        const CHUNK: usize = 32;
        let mut buf = [0u8; CHUNK + 1];
        for chunk in data.chunks(CHUNK) {
            buf[0] = 0x40;
            buf[1..1 + chunk.len()].copy_from_slice(chunk);
            let _ = self.i2c.write(ADDR, &buf[..1 + chunk.len()]);
        }
    }

    fn init(&mut self) {
        for b in [
            0xAE, // display off
            0x20, 0x00, // horizontal addressing
            0x40, // start line
            0xA0, // segment remap (180° — upside down)
            0xA8, 0x27, // multiplex ratio (40-1)
            0xC0, // COM scan inc (180° — upside down)
            0xD3, 0x00, // offset
            0xDA, 0x12, // COM pins (same as upstream driver for height != 32)
            0xD5, 0x80, // clock
            0xD9, 0xF1, // precharge
            0xDB, 0x30, // VCOM
            0x81, 0xFF, // contrast
            0xA4, // resume RAM
            0xA6, // normal (not invert)
            0x8D, 0x14, // charge pump on
            0xAF, // display on
        ] {
            self.cmd(b);
        }
    }

    pub fn show(&mut self, frame: &[u8; FRAME_LEN]) {
        self.cmd(0x21); // column addr
        self.cmd(COL_OFFSET);
        self.cmd(COL_OFFSET + (WIDTH - 1) as u8);
        self.cmd(0x22); // page addr
        self.cmd(0);
        self.cmd((HEIGHT / 8 - 1) as u8);
        self.write_data(frame);
    }
}
