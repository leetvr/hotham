use std::collections::HashSet;

use winit::event::{ElementState, VirtualKeyCode};

#[derive(Debug, Clone)]
pub enum HothamInputEvent {
    KeyboardInput {
        state: ElementState,
        key_code: Option<VirtualKeyCode>,
    },
    MouseInput {
        x: f64,
        y: f64,
    },
}

#[derive(Default, Debug)]
pub struct Inputs {
    pub pressed: HashSet<VirtualKeyCode>,
}

impl Inputs {
    pub fn process_event(&mut self, input_event: HothamInputEvent) {
        match input_event {
            HothamInputEvent::KeyboardInput { state, key_code } => match key_code {
                Some(key) => {
                    let _ = match state {
                        ElementState::Pressed => self.pressed.insert(key),
                        ElementState::Released => self.pressed.remove(&key),
                    };
                }
                _ => {}
            },
            _ => {}
        }
    }
}
