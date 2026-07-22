//! BOOT button on GPIO9 (active-low) — jump / restart.

use esp_hal::gpio::Input;

pub struct BootButton<'d> {
    pin: Input<'d>,
    was_down: bool,
}

impl<'d> BootButton<'d> {
    pub fn new(pin: Input<'d>) -> Self {
        Self {
            pin,
            was_down: false,
        }
    }

    /// True while BOOT is held (active-low).
    pub fn is_down(&self) -> bool {
        self.pin.is_low()
    }

    /// True on the frame the button goes from released → pressed.
    pub fn pressed_edge(&mut self) -> bool {
        let down = self.is_down();
        let edge = down && !self.was_down;
        self.was_down = down;
        edge
    }
}
