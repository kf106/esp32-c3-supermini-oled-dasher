//! Persist current level index across power cycles via SPI flash.
//!
//! Reads use the memory-mapped flash window (no ROM calls). Erase/write run
//! from `.rwtext` (IRAM) under a critical section.

use crate::level::LEVEL_COUNT;
use critical_section::{acquire, release};

const SECTOR_SIZE: u32 = 4096;
const MAGIC: u32 = 0x4853_4144; // "DASH" LE
const VERSION: u32 = 1;
/// Cached / DROM flash mapping on ESP32-C3.
const DROM_BASE: usize = 0x3C00_0000;

/// ESP32-C3 ROM entry points (from esp32c3.rom.ld).
const ROM_WRITE: usize = 0x4000_012c;
const ROM_ERASE_SECTOR: usize = 0x4000_0128;
const ROM_UNLOCK: usize = 0x4000_0140;

type RomWrite = unsafe extern "C" fn(u32, *const u32, u32) -> i32;
type RomErase = unsafe extern "C" fn(u32) -> i32;
type RomUnlock = unsafe extern "C" fn() -> i32;

static mut CURRENT: u8 = 0;
static mut LOADED: bool = false;
static mut DIRTY: bool = false;
/// Byte offset of the save sector; filled on first load/save.
static mut SAVE_OFF: u32 = 0;

fn ensure_save_off() -> u32 {
    unsafe {
        if SAVE_OFF == 0 {
            SAVE_OFF = save_offset();
        }
        SAVE_OFF
    }
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
    // Flash size nibble lives in the image/chip header at offset 3.
    let b = unsafe { core::ptr::read_volatile((DROM_BASE + 3) as *const u8) };
    let mb = match b & 0xf0 {
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

fn read_record_mapped() -> Option<u8> {
    let off = ensure_save_off() as usize;
    let p = (DROM_BASE + off) as *const u32;
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
            CURRENT = read_record_mapped().unwrap_or(0);
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
    let (level, off) = unsafe {
        if !DIRTY {
            return;
        }
        DIRTY = false;
        let level = CURRENT.min((LEVEL_COUNT - 1) as u8);
        let off = if SAVE_OFF == 0 {
            // 4MB default if never loaded; matches these boards.
            4 * 1024 * 1024 - SECTOR_SIZE
        } else {
            SAVE_OFF
        };
        (level, off)
    };

    let level_u = u32::from(level);
    let magic = MAGIC;
    let version = VERSION;
    let sum = checksum(magic, version, level_u);
    let words: [u32; 4] = [magic, version, level_u, sum];
    let _ = unsafe { flash_commit(off, words.as_ptr()) };
}
