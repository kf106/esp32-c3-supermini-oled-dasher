//! Geometry Dash–style runner on ESP32-C3 Supermini OLED (72×40).
//! Jump with the BOOT button (GPIO9).

#![no_std]
#![no_main]

mod font;
mod framebuf;
mod game;
mod input;
mod led;
mod level;
mod save;
mod splash;
mod ssd1306;

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    entry,
    gpio::{Input, Level, Output, Pull},
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
/// Frames to wait after splash before ROM save read/write (flash I/O near boot hangs).
const FLASH_SETTLE_FRAMES: u32 = 60;

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
    // Blue LED: active-low, start off (high).
    let mut led = Output::new(io.pins.gpio8, Level::High);

    let mut frame = [0u8; FRAME_LEN];

    // Splash: hold BOOT for RESET_HOLD_MS to wipe progress (no flash I/O here).
    splash::draw(&mut frame);
    display.show(&frame);

    let polls = SPLASH_MS / SPLASH_POLL_MS;
    let mut held = 0u32;
    let mut wiped = false;
    for _ in 0..polls {
        delay.delay_millis(SPLASH_POLL_MS);
        if boot.is_down() {
            held = held.saturating_add(SPLASH_POLL_MS);
            if !wiped && held >= RESET_HOLD_MS {
                wiped = true;
            }
        } else {
            held = 0;
        }
        let _ = boot.pressed_edge();
    }
    boot.sync();

    if wiped {
        save::clear_progress_ram();
        splash::draw_erased(&mut frame);
        display.show(&frame);
        delay.delay_millis(400);
    }

    // Mmap read of SAVE_PAGE — no ROM SPI (ROM reads hang this board).
    let mut game = if wiped {
        Game::new(0)
    } else if save::is_all_clear() {
        Game::all_clear()
    } else {
        Game::new(save::load_level())
    };
    let mut persist_in = if wiped { FLASH_SETTLE_FRAMES } else { 0 };
    let mut hold_ms = 0u32;

    game.draw(&mut frame);
    display.show(&frame);

    loop {
        if persist_in == 1 {
            save::flush();
        }
        if persist_in > 0 {
            persist_in -= 1;
        }

        if game.is_complete() {
            // Hold BOOT 2s on the complete screen to wipe → level 1.
            if boot.is_down() {
                hold_ms = hold_ms.saturating_add(TICK_MS);
                if hold_ms >= RESET_HOLD_MS {
                    save::clear_progress_ram();
                    splash::draw_erased(&mut frame);
                    display.show(&frame);
                    delay.delay_millis(400);
                    game.restart_from_wipe();
                    persist_in = FLASH_SETTLE_FRAMES;
                    hold_ms = 0;
                    boot.sync();
                }
            } else {
                hold_ms = 0;
            }
            let _ = boot.pressed_edge();
            let _ = game.update(false);
        } else {
            hold_ms = 0;
            let jump = boot.pressed_edge();
            if game.update(jump) {
                led::flash_three(&mut led, &delay);
                persist_in = FLASH_SETTLE_FRAMES;
            }
        }
        game.draw(&mut frame);
        display.show(&frame);
        delay.delay_millis(TICK_MS);
    }
}
