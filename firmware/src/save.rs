//! Persist current level index across power cycles via SPI flash.
//!
//! Erase/write must run from `.rwtext` (IRAM) under a critical section. Calling
//! the ROM flash drivers from XIP code (the previous approach) stalls instruction
//! fetch and leaves the OLED showing static.

use crate::level::LEVEL_COUNT;
use critical_section::{acquire, release};

const SECTOR_SIZE: u32 = 4096;
const MAGIC: u32 = 0x4853_4144; // "DASH" LE
const VERSION: u32 = 1;

/// ESP32-C3 ROM entry points (from esp32c3.rom.ld).
const ROM_READ: usize = 0x4000_0130;
const ROM_WRITE: usize = 0x4000_012c;
const ROM_ERASE_SECTOR: usize = 0x4000_0128;
const ROM_UNLOCK: usize = 0x4000_0140;

type RomRead = unsafe extern "C" fn(u32, *mut u32, u32) -> i32;
type RomWrite = unsafe extern "C" fn(u32, *const u32, u32) -> i32;
type RomErase = unsafe extern "C" fn(u32) -> i32;
type RomUnlock = unsafe extern "C" fn() -> i32;

static mut CURRENT: u8 = 0;
static mut LOADED: bool = false;

#[inline(always)]
#[link_section = ".rwtext"]
fn rom_read(addr: u32, data: *mut u32, len: u32) -> i32 {
    let f: RomRead = unsafe { core::mem::transmute(ROM_READ) };
    unsafe { f(addr, data, len) }
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

fn flash_capacity_bytes() -> u32 {
    let mut word = [0u32; 1];
    let rc = rom_read(0, word.as_mut_ptr(), 4);
    if rc != 0 {
        return 4 * 1024 * 1024;
    }
    let mb = match word[0].to_le_bytes()[3] & 0xf0 {
        0x00 => 1,
        0x10 => 2,
        0x20 => 4,
        0x30 => 8,
        0x40 => 16,
        _ => 4,
    };
    mb * 1024 * 1024
}

fn save_offset() -> u32 {
    flash_capacity_bytes().saturating_sub(SECTOR_SIZE)
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
    let mut words = [0u32; 4];
    let off = save_offset();
    if rom_read(off, words.as_mut_ptr(), 16) != 0 {
        return None;
    }
    let magic = words[0];
    let version = words[1];
    let level = words[2];
    let sum = words[3];
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
        }
        CURRENT.min((LEVEL_COUNT - 1) as u8)
    }
}

/// Persist level index (clamped). Survives reset and power-cycle.
pub fn save_level(level: u8) {
    let level = level.min((LEVEL_COUNT - 1) as u8);
    unsafe {
        if LOADED && CURRENT == level {
            return;
        }
        CURRENT = level;
        LOADED = true;
    }

    let level_u = u32::from(level);
    let magic = MAGIC;
    let version = VERSION;
    let sum = checksum(magic, version, level_u);
    let words: [u32; 4] = [magic, version, level_u, sum];
    let off = save_offset();
    let _ = unsafe { flash_commit(off, words.as_ptr()) };
}

/// Wipe progress back to level 1 (index 0) and persist.
pub fn clear_progress() {
    save_level(0);
}
