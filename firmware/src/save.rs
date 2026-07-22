//! Persist current level index across power cycles via SPI flash.
//!
//! The save record lives in a 4KB `.rodata` page so boot can **read** it through
//! the DROM mmap (no ROM SPI). Erase/write still use ROM calls from `.rwtext`
//! (IRAM). Flash address of that page is computed by `flash.sh` into
//! `save_flash_offset.rs` (must match the ESP image segment layout — not
//! `vaddr - 0x3C00_0000 + 0x10000`).

use crate::level::LEVEL_COUNT;
use critical_section::{acquire, release};

const SECTOR_SIZE: u32 = 4096;
const MAGIC: u32 = 0x4853_4144; // "DASH" LE
const VERSION: u32 = 1;

/// ESP32-C3 ROM entry points (from esp32c3.rom.ld).
const ROM_WRITE: usize = 0x4000_012c;
const ROM_ERASE_SECTOR: usize = 0x4000_0128;
const ROM_UNLOCK: usize = 0x4000_0140;

type RomWrite = unsafe extern "C" fn(u32, *const u32, u32) -> i32;
type RomErase = unsafe extern "C" fn(u32) -> i32;
type RomUnlock = unsafe extern "C" fn() -> i32;

/// One flash sector, kept in DROM so it is MMU-mapped for reads after boot.
#[repr(C, align(4096))]
struct SavePage {
    record: [u32; 4],
    _pad: [u8; SECTOR_SIZE as usize - 16],
}

#[no_mangle]
#[link_section = ".rodata"]
#[used]
static SAVE_PAGE: SavePage = SavePage {
    record: [0xFFFF_FFFF; 4],
    _pad: [0xFF; SECTOR_SIZE as usize - 16],
};

static mut CURRENT: u8 = 0;
static mut LOADED: bool = false;
static mut DIRTY: bool = false;

include!("save_flash_offset.rs");

fn save_offset() -> u32 {
    SAVE_FLASH_OFFSET
}

#[inline(always)]
#[link_section = ".rwtext"]
fn rom_write(addr: u32, data: *const u32, len: u32) -> i32 {
    let f: RomWrite = unsafe { core::mem::transmute(ROM_WRITE) };
    unsafe { f(addr, data, len) }
}

#[inline(always)]
#[link_section = ".rwtext"]
fn rom_erase(sector: u32) -> i32 {
    let f: RomErase = unsafe { core::mem::transmute(ROM_ERASE_SECTOR) };
    unsafe { f(sector) }
}

#[inline(always)]
#[link_section = ".rwtext"]
fn rom_unlock() -> i32 {
    let f: RomUnlock = unsafe { core::mem::transmute(ROM_UNLOCK) };
    unsafe { f() }
}

fn checksum(magic: u32, version: u32, level: u32) -> u32 {
    magic ^ version ^ level ^ 0xA5A5_C3C3
}

/// Erase the save sector and write the 16-byte record. Entire body lives in IRAM.
#[inline(never)]
#[link_section = ".rwtext"]
unsafe fn flash_commit(off: u32, words: *const u32) -> bool {
    let cs = acquire();
    let ok = {
        if rom_unlock() != 0 {
            false
        } else if rom_erase(off / SECTOR_SIZE) != 0 {
            false
        } else {
            rom_write(off, words, 16) == 0
        }
    };
    release(cs);
    ok
}

fn read_record() -> Option<u8> {
    // Mapped .rodata — safe at boot (no ROM SPI).
    let p = core::ptr::addr_of!(SAVE_PAGE.record) as *const u32;
    let magic = unsafe { core::ptr::read_volatile(p) };
    let version = unsafe { core::ptr::read_volatile(p.add(1)) };
    let level = unsafe { core::ptr::read_volatile(p.add(2)) };
    let sum = unsafe { core::ptr::read_volatile(p.add(3)) };
    if magic != MAGIC || version != VERSION {
        return None;
    }
    if sum != checksum(magic, version, level) {
        return None;
    }
    if level as usize >= LEVEL_COUNT {
        return None;
    }
    Some(level as u8)
}

/// Load saved level index (0..LEVEL_COUNT). Defaults to 0 on missing/invalid data.
pub fn load_level() -> u8 {
    unsafe {
        if !LOADED {
            CURRENT = read_record().unwrap_or(0);
            LOADED = true;
            DIRTY = false;
        }
        CURRENT.min((LEVEL_COUNT - 1) as u8)
    }
}

/// RAM-only update (no SPI). Call [`flush`] later to persist.
pub fn set_level_ram(level: u8) {
    unsafe {
        CURRENT = level.min((LEVEL_COUNT - 1) as u8);
        LOADED = true;
        DIRTY = true;
    }
}

/// Persist level index (clamped). Survives reset and power-cycle.
pub fn save_level(level: u8) {
    set_level_ram(level);
    flush();
}

/// Wipe progress back to level 1 (index 0) in RAM. Call [`flush`] to persist.
pub fn clear_progress_ram() {
    set_level_ram(0);
}

/// Wipe + persist immediately.
#[allow(dead_code)]
pub fn clear_progress() {
    clear_progress_ram();
    flush();
}

/// Write dirty RAM progress to flash (safe to call after the display is running).
#[inline(never)]
#[link_section = ".rwtext"]
pub fn flush() {
    let level = unsafe {
        if !DIRTY {
            return;
        }
        DIRTY = false;
        CURRENT.min((LEVEL_COUNT - 1) as u8)
    };
    let off = save_offset();

    let level_u = u32::from(level);
    let magic = MAGIC;
    let version = VERSION;
    let sum = checksum(magic, version, level_u);
    let words: [u32; 4] = [magic, version, level_u, sum];
    let _ = unsafe { flash_commit(off, words.as_ptr()) };
}
