//! Level progress — RAM only for now.
//!
//! SPI flash erase/write from XIP code corrupts the running image fetch and
//! leaves the OLED showing static. Persistence is disabled until a safe
//! IRAM + cache-suspend path is proven on device.

use crate::level::LEVEL_COUNT;

static mut CURRENT: u8 = 0;

/// Load saved level index (0..LEVEL_COUNT).
pub fn load_level() -> u8 {
    let level = unsafe { CURRENT };
    level.min((LEVEL_COUNT - 1) as u8)
}

/// Persist level index in RAM (lost on power cycle).
pub fn save_level(level: u8) {
    unsafe {
        CURRENT = level.min((LEVEL_COUNT - 1) as u8);
    }
}
