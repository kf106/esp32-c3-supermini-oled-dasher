//! Persist current level index across power cycles via SPI flash.
//!
//! Design constraints on this board:
//! - ROM SPI **reads** hang (even with cache suspend).
//! - ROM **erase** of a `.rodata` page bricks the next boot (black screen).
//!
//! So: keep a 4KB [`SAVE_PAGE`] in `.rodata` for **mmap reads**, and persist by
//! **append-only** `rom_write` into free `0xFF` slots (no erase). Flash can only
//! clear bits; erased flash is `0xFF`, so programming a fresh slot is safe.
//! `flash.sh` clears the app `hash_appended` flag so these in-image writes do not
//! fail the bootloader SHA-256 check (which would otherwise black-screen on reboot).
//!
//! Level field: `0..LEVEL_COUNT-1` = in progress, [`ALL_CLEAR`] (999) = finished.

use crate::level::LEVEL_COUNT;
use critical_section::{acquire, release};

const SECTOR_SIZE: u32 = 4096;
const SLOT_BYTES: u32 = 16;
const SLOT_COUNT: u32 = SECTOR_SIZE / SLOT_BYTES; // 256
const MAGIC: u32 = 0x4853_4144; // "DASH" LE
const VERSION: u32 = 1;
const EMPTY: u32 = 0xFFFF_FFFF;
/// Saved when every course is cleared — boot shows the complete splash.
pub const ALL_CLEAR: u32 = 999;

/// ESP32-C3 ROM entry points (esp32c3.rom.ld).
const ROM_WRITE: usize = 0x4000_012c;
const ROM_UNLOCK: usize = 0x4000_0140;
const CACHE_SUSPEND_ICACHE: usize = 0x4000_0524;
const CACHE_RESUME_ICACHE: usize = 0x4000_0528;
const CACHE_INVALIDATE_ICACHE_ALL: usize = 0x4000_04d8;

type RomWrite = unsafe extern "C" fn(u32, *const u32, u32) -> i32;
type RomUnlock = unsafe extern "C" fn() -> i32;
type CacheSuspend = unsafe extern "C" fn() -> u32;
type CacheResume = unsafe extern "C" fn(u32);
type CacheInvalidate = unsafe extern "C" fn();

/// One flash sector in DROM — MMU-mapped for safe reads at boot.
#[repr(C, align(4096))]
struct SavePage {
    bytes: [u8; SECTOR_SIZE as usize],
}

#[no_mangle]
#[link_section = ".rodata"]
#[used]
static SAVE_PAGE: SavePage = SavePage {
    bytes: [0xFF; SECTOR_SIZE as usize],
};

static mut CURRENT: u32 = 0;
static mut LOADED: bool = false;
static mut DIRTY: bool = false;
/// Next journal slot to program (RAM); avoids relying on post-write mmap cache.
static mut NEXT_SLOT: u32 = 0;

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
fn rom_unlock() -> i32 {
    let f: RomUnlock = unsafe { core::mem::transmute(ROM_UNLOCK) };
    unsafe { f() }
}

#[inline(always)]
#[link_section = ".rwtext"]
unsafe fn cache_suspend() -> u32 {
    let f: CacheSuspend = core::mem::transmute(CACHE_SUSPEND_ICACHE);
    f()
}

#[inline(always)]
#[link_section = ".rwtext"]
unsafe fn cache_resume(state: u32) {
    let inv: CacheInvalidate = core::mem::transmute(CACHE_INVALIDATE_ICACHE_ALL);
    let resume: CacheResume = core::mem::transmute(CACHE_RESUME_ICACHE);
    inv();
    resume(state);
}

fn checksum(magic: u32, version: u32, level: u32) -> u32 {
    magic ^ version ^ level ^ 0xA5A5_C3C3
}

fn slot_ptr(index: u32) -> *const u32 {
    let base = core::ptr::addr_of!(SAVE_PAGE.bytes) as *const u8;
    unsafe { base.add((index * SLOT_BYTES) as usize) as *const u32 }
}

fn parse_slot(index: u32) -> Option<u32> {
    let p = slot_ptr(index);
    let magic = unsafe { core::ptr::read_volatile(p) };
    if magic == EMPTY {
        return None;
    }
    let version = unsafe { core::ptr::read_volatile(p.add(1)) };
    let level = unsafe { core::ptr::read_volatile(p.add(2)) };
    let sum = unsafe { core::ptr::read_volatile(p.add(3)) };
    if magic != MAGIC || version != VERSION {
        return None;
    }
    if sum != checksum(magic, version, level) {
        return None;
    }
    if level == ALL_CLEAR {
        return Some(ALL_CLEAR);
    }
    if level as usize >= LEVEL_COUNT {
        return None;
    }
    Some(level)
}

/// Latest valid record via mmap (no ROM SPI).
fn read_record() -> Option<u32> {
    let mut found = None;
    let mut next = SLOT_COUNT;
    for i in 0..SLOT_COUNT {
        let magic = unsafe { core::ptr::read_volatile(slot_ptr(i)) };
        if magic == EMPTY {
            next = i;
            break;
        }
        next = i + 1;
        if let Some(level) = parse_slot(i) {
            found = Some(level);
        }
    }
    unsafe {
        NEXT_SLOT = next.min(SLOT_COUNT);
    }
    found
}

/// Byte offset of the next empty slot within the page, if any.
fn next_free_rel() -> Option<u32> {
    let slot = unsafe { NEXT_SLOT };
    if slot >= SLOT_COUNT {
        return None;
    }
    Some(slot * SLOT_BYTES)
}

/// Append one record into a free `0xFF` slot. No erase (avoids bricking).
#[inline(never)]
#[link_section = ".rwtext"]
unsafe fn flash_append(words: *const u32) -> bool {
    let Some(rel) = next_free_rel() else {
        // Journal full — drop the write rather than erase .rodata.
        return false;
    };
    let addr = save_offset() + rel;
    let cs = acquire();
    let state = cache_suspend();
    let ok = rom_unlock() == 0 && rom_write(addr, words, 16) == 0;
    cache_resume(state);
    release(cs);
    if ok {
        NEXT_SLOT = (rel / SLOT_BYTES) + 1;
    }
    ok
}

fn ensure_loaded() {
    unsafe {
        if !LOADED {
            CURRENT = read_record().unwrap_or(0);
            LOADED = true;
            DIRTY = false;
        }
    }
}

pub fn is_all_clear() -> bool {
    ensure_loaded();
    unsafe { CURRENT == ALL_CLEAR }
}

pub fn load_level() -> u8 {
    ensure_loaded();
    unsafe {
        if CURRENT == ALL_CLEAR {
            (LEVEL_COUNT - 1) as u8
        } else {
            (CURRENT as u8).min((LEVEL_COUNT - 1) as u8)
        }
    }
}

pub fn set_level_ram(level: u8) {
    unsafe {
        CURRENT = u32::from(level.min((LEVEL_COUNT - 1) as u8));
        LOADED = true;
        DIRTY = true;
    }
}

pub fn set_all_clear_ram() {
    unsafe {
        CURRENT = ALL_CLEAR;
        LOADED = true;
        DIRTY = true;
    }
}

#[allow(dead_code)]
pub fn save_all_clear() {
    set_all_clear_ram();
    flush();
}

#[allow(dead_code)]
pub fn save_level(level: u8) {
    set_level_ram(level);
    flush();
}

pub fn clear_progress_ram() {
    set_level_ram(0);
}

#[allow(dead_code)]
pub fn clear_progress() {
    clear_progress_ram();
    flush();
}

/// Append dirty RAM progress (deferred from `main`; no sector erase).
#[inline(never)]
#[link_section = ".rwtext"]
pub fn flush() {
    let level = unsafe {
        if !DIRTY {
            return;
        }
        DIRTY = false;
        if CURRENT == ALL_CLEAR {
            ALL_CLEAR
        } else {
            CURRENT.min((LEVEL_COUNT - 1) as u32)
        }
    };
    let magic = MAGIC;
    let version = VERSION;
    let sum = checksum(magic, version, level);
    let words: [u32; 4] = [magic, version, level, sum];
    let _ = unsafe { flash_append(words.as_ptr()) };
}
