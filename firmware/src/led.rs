//! On-board blue LED (GPIO 8, active-low).

use esp_hal::{delay::Delay, gpio::Output};

/// Three quick flashes (on/off), then leave the LED off.
pub fn flash_three(led: &mut Output<'_>, delay: &Delay) {
    for _ in 0..3 {
        led.set_low(); // on
        delay.delay_millis(60);
        led.set_high(); // off
        delay.delay_millis(60);
    }
}
