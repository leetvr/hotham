use std::collections::HashSet;

use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub pressed: HashSet<VirtualKeyCode>,
}

impl Inputs {
    pub fn process_event(&mut self, keyboard_input: KeyboardInput) {
        if let Some(key) = keyboard_input.virtual_keycode {
            let _ = match keyboard_input.state {
                ElementState::Pressed => self.pressed.insert(key),
                ElementState::Released => self.pressed.remove(&key),
            };
        }
    }
}
