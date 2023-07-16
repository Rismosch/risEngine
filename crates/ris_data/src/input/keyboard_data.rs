use std::time::Instant;

use sdl2::keyboard::Scancode;

use super::buttons::Buttons;

#[derive(Clone)]
pub struct KeyboardData {
    pub buttons: Buttons,
    pub keymask: [Scancode; 32],

    pub crash_timestamp: Instant,
    pub restart_timestamp: Instant,
}

impl KeyboardData {
    pub fn new(keymask: [Scancode; 32]) -> Self {
        Self {
            buttons: Buttons::default(),
            keymask,
            crash_timestamp: Instant::now(),
            restart_timestamp: Instant::now(),
        }
    }
}

impl Default for KeyboardData {
    fn default() -> Self {
        Self::new([Scancode::A; 32])
    }
}
