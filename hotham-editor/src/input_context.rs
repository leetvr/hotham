use glam::{Quat, Vec3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

use crate::camera::Pose;

const INPUT_SPEED: f32 = 10.;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputContext {
    keyboard_state: KeyboardState,
    mouse_state: MouseState,
}

impl Default for InputContext {
    fn default() -> Self {
        Self {
            keyboard_state: KeyboardState::Idle,
            mouse_state: MouseState::Idle,
        }
    }
}

impl InputContext {
    pub fn update(&mut self, delta_time: f32, keyboard_input: &[KeyboardInput], pose: &mut Pose) {
        if self.keyboard_state != KeyboardState::Idle || !keyboard_input.is_empty() {
            dbg!(keyboard_input);
            dbg!(&self.keyboard_state);
        }
        let movement_speed = INPUT_SPEED * delta_time;

        // process event queue
        for event in keyboard_input {
            let (state, keycode) = (event.state, event.virtual_keycode.unwrap());
            //safe as we only receive events with a keycode

            let next_state = match (&self.keyboard_state, state, keycode) {
                (_, ElementState::Pressed, _) => KeyboardState::HoldingKey(keycode),
                _ => KeyboardState::Idle,
            };

            self.keyboard_state = next_state;
        }

        match self.keyboard_state {
            KeyboardState::HoldingKey(key) => update_pose(key, movement_speed, pose),
            _ => {}
        };
    }
}

fn update_pose(key: VirtualKeyCode, movement_speed: f32, pose: &mut Pose) {
    let position = &mut pose.position;
    let orientation = pose.orientation;
    // get the forward vector rotated by the camera rotation quaternion
    let forward = orientation * -Vec3::Z;
    // get the right vector rotated by the camera rotation quaternion
    let right = orientation * Vec3::X;
    let up = Vec3::Y;

    match key {
        winit::event::VirtualKeyCode::W => {
            position.x += forward.x * movement_speed;
            position.y += forward.y * movement_speed;
            position.z += forward.z * movement_speed;
        }
        winit::event::VirtualKeyCode::S => {
            position.x -= forward.x * movement_speed;
            position.y -= forward.y * movement_speed;
            position.z -= forward.z * movement_speed;
        }
        winit::event::VirtualKeyCode::A => {
            position.x -= right.x * movement_speed;
            position.y -= right.y * movement_speed;
            position.z -= right.z * movement_speed;
        }
        winit::event::VirtualKeyCode::D => {
            position.x += right.x * movement_speed;
            position.y += right.y * movement_speed;
            position.z += right.z * movement_speed;
        }
        winit::event::VirtualKeyCode::Space => {
            position.y += up.y * movement_speed;
        }
        winit::event::VirtualKeyCode::LShift => {
            position.y -= up.y * movement_speed;
        }
        _ => {}
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum KeyboardState {
    Idle,
    HoldingKey(VirtualKeyCode),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MouseState {
    Idle,
    HoldingLeftClick,
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use winit::event::{ElementState, KeyboardInput, ModifiersState, VirtualKeyCode};

    use crate::{camera::Pose, input_context::INPUT_SPEED};

    use super::InputContext;

    #[test]
    pub fn test_keyboard_input() {
        let mut input_context = InputContext::default();
        let mut pose = Pose {
            position: Default::default(),
            orientation: Default::default(),
        };

        // press w
        input_context.update(1.0, &[press(VirtualKeyCode::W)], &mut pose);
        assert_eq!(pose.position, [0., 0., -INPUT_SPEED].into());

        // keep holding it
        input_context.update(1.0, &[], &mut pose);
        assert_eq!(pose.position, [0., 0., -INPUT_SPEED * 2.0].into());

        // release
        input_context.update(1.0, &[release(VirtualKeyCode::W)], &mut pose);
        assert_eq!(pose.position, [0., 0., -INPUT_SPEED * 2.0].into());
    }

    fn press(virtual_code: VirtualKeyCode) -> KeyboardInput {
        KeyboardInput {
            scancode: 0,
            state: ElementState::Pressed,
            virtual_keycode: Some(virtual_code),
            modifiers: ModifiersState::empty(),
        }
    }

    fn release(virtual_code: VirtualKeyCode) -> KeyboardInput {
        KeyboardInput {
            scancode: 0,
            state: ElementState::Released,
            virtual_keycode: Some(virtual_code),
            modifiers: ModifiersState::empty(),
        }
    }
}
