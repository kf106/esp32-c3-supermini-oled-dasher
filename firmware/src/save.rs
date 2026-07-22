//! Persist current level index across power cycles via SPI flash ROM.

use crate::level::LEVEL_COUNT;

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

#[inline(always)]
fn rom_read() -> RomRead {
    unsafe { core::mem::transmute(ROM_READ) }
}

#[inline(always)]
fn rom_write() -> RomWrite {
    unsafe { core::mem::transmute(ROM_WRITE) }
}

#[inline(always)]
fn rom_erase() -> RomErase {
    unsafe { core::mem::transmute(ROM_ERASE_SECTOR) }
}

#[inline(always)]
fn rom_unlock() -> RomUnlock {
    unsafe { core::mem::transmute(ROM_UNLOCK) }
}

fn flash_capacity_bytes() -> u32 {
    let mut word = [0u32; 1];
    let rc = unsafe { rom_read()(0, word.as_mut_ptr(), 4) };
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

/// Load saved level index (0..LEVEL_COUNT). Defaults to 0 on missing/invalid data.
pub fn load_level() -> u8 {
    let mut words = [0u32; 4];
    let off = save_offset();
    let rc = unsafe { rom_read()(off, words.as_mut_ptr(), 16) };
    if rc != 0 {
        return 0;
    }
    let magic = words[0];
    let version = words[1];
    let level = words[2];
    let sum = words[3];
    if magic != MAGIC || version != VERSION {
        return 0;
    }
    if sum != checksum(magic, version, level) {
        return 0;
    }
    if level as usize >= LEVEL_COUNT {
        return 0;
    }
    level as u8
}

/// Persist level index (clamped to 0..LEVEL_COUNT).
pub fn save_level(level: u8) {
    let level = u32::from(level.min((LEVEL_COUNT - 1) as u8));
    let magic = MAGIC;
    let version = VERSION;
    let sum = checksum(magic, version, level);
    let off = save_offset();
    let sector = off / SECTOR_SIZE;
    let words: [u32; 4] = [magic, version, level, sum];

    unsafe {
        let _ = rom_unlock()();
        let _ = rom_erase()(sector);
        let _ = rom_write()(off, words.as_ptr(), 16);
    }
}
