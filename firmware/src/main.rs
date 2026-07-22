//! Geometry Dash–style runner on ESP32-C3 Supermini OLED (72×40).
//! Jump with the BOOT button (GPIO9).

#![no_std]
#![no_main]

mod font;
mod framebuf;
mod game;
mod input;
mod level;
mod save;
mod splash;
mod ssd1306;

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    entry,
    gpio::{Input, Pull},
    i2c::I2c,
    prelude::*,
    Config,
};
use game::Game;
use input::BootButton;
use splash::{RESET_HOLD_MS, SPLASH_MS};
use ssd1306::{Display, FRAME_LEN};

const TICK_MS: u32 = 33;
const SPLASH_POLL_MS: u32 = 20;

const ESP_APP_DESC_MAGIC_WORD: u32 = 0xABCD5432;

#[repr(C)]
struct EspAppDesc {
    magic_word: u32,
    secure_version: u32,
    reserv1: [u32; 2],
    version: [u8; 32],
    project_name: [u8; 32],
    time: [u8; 16],
    date: [u8; 16],
    idf_ver: [u8; 32],
    app_elf_sha256: [u8; 32],
    min_efuse_blk_rev_full: u16,
    max_efuse_blk_rev_full: u16,
    mmu_page_size: u8,
    reserv3: [u8; 3],
    reserv2: [u32; 18],
}

#[unsafe(export_name = "esp_app_desc")]
#[unsafe(link_section = ".rodata")]
#[used]
static ESP_APP_DESC: EspAppDesc = EspAppDesc {
    magic_word: ESP_APP_DESC_MAGIC_WORD,
    secure_version: 0,
    reserv1: [0; 2],
    version: cstr_32("0.1.2"),
    project_name: cstr_32("oled-dash"),
    time: cstr_16("00:00:00"),
    date: cstr_16("1970-01-01"),
    idf_ver: cstr_32("esp-hal"),
    app_elf_sha256: [0; 32],
    min_efuse_blk_rev_full: 0,
    max_efuse_blk_rev_full: u16::MAX,
    mmu_page_size: 0,
    reserv3: [0; 3],
    reserv2: [0; 18],
};

const fn cstr_32(s: &str) -> [u8; 32] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 32];
    let mut i = 0;
    while i < bytes.len() && i < 31 {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

const fn cstr_16(s: &str) -> [u8; 16] {
    let bytes = s.as_bytes();
    let mut out = [0u8; 16];
    let mut i = 0;
    while i < bytes.len() && i < 15 {
        out[i] = bytes[i];
        i += 1;
    }
    out
}

#[entry]
fn main() -> ! {
    let peripherals = esp_hal::init(Config::default());

    let io = esp_hal::gpio::Io::new(peripherals.GPIO, peripherals.IO_MUX);
    let i2c = I2c::new(peripherals.I2C0, io.pins.gpio5, io.pins.gpio6, 400.kHz());
    let mut display = Display::new(i2c);
    let delay = Delay::new();
    let mut boot = BootButton::new(Input::new(io.pins.gpio9, Pull::Up));

    let mut frame = [0u8; FRAME_LEN];

    // Splash: hold BOOT for RESET_HOLD_MS to wipe progress (after ROM boot, so
    // this does not enter download mode).
    splash::draw(&mut frame);
    display.show(&frame);
    let mut elapsed = 0u32;
    let mut held = 0u32;
    let mut wiped = false;
    while elapsed < SPLASH_MS {
        delay.delay_millis(SPLASH_POLL_MS);
        elapsed = elapsed.saturating_add(SPLASH_POLL_MS);
        if boot.is_down() {
            held = held.saturating_add(SPLASH_POLL_MS);
            if !wiped && held >= RESET_HOLD_MS {
                save::clear_progress();
                splash::draw_erased(&mut frame);
                display.show(&frame);
                wiped = true;
            }
        } else {
            held = 0;
        }
        let _ = boot.pressed_edge(); // keep edge state in sync
    }

    let saved = save::load_level();
    let mut game = Game::new(saved);

    loop {
        let jump = boot.pressed_edge();
        game.update(jump);
        game.draw(&mut frame);
        display.show(&frame);
        delay.delay_millis(TICK_MS);
    }
}
