use std::collections::HashSet;

use winit::{event::ElementState, keyboard::PhysicalKey};

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub pressed: HashSet<winit::keyboard::KeyCode>,
}

impl Inputs {
    pub fn process_event(&mut self, keyboard_input: winit::event::KeyEvent) {
        match (keyboard_input.physical_key, keyboard_input.state) {
            (PhysicalKey::Code(key_code), ElementState::Pressed) => {
                self.pressed.insert(key_code);
            }
            (PhysicalKey::Code(key_code), ElementState::Released) => {
                self.pressed.remove(&key_code);
            }
            _ => {}
        }
    }
}
