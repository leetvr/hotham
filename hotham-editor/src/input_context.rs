use glam::{Vec2, Vec3};
use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

use crate::{camera::Pose, MouseInput};

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
            mouse_state: MouseState::HoldingLeftClick,
        }
    }
}

impl InputContext {
    pub fn update(
        &mut self,
        delta_time: f32,
        keyboard_input: &[KeyboardInput],
        mouse_input: &[MouseInput],
        pose: &mut Pose,
    ) {
        let movement_speed = INPUT_SPEED * delta_time;

        let mut mouse_motion = Vec2::ZERO;
        for event in mouse_input {
            match event {
                MouseInput::LeftClickPressed => self.mouse_state = MouseState::HoldingLeftClick,
                MouseInput::LeftClickReleased => self.mouse_state = MouseState::Idle,
                MouseInput::MouseMoved(delta) => {
                    if self.mouse_state == MouseState::HoldingLeftClick {
                        mouse_motion += *delta;
                    }
                }
            };
        }
        handle_mouse_movement(mouse_motion * movement_speed, pose);

        let mut keyboard_motion = Vec3::ZERO;
        for event in keyboard_input {
            //safe as we only receive events with a keycode
            let (state, key) = (event.state, event.virtual_keycode.unwrap());

            match state {
                ElementState::Pressed => {
                    self.keyboard_state = KeyboardState::HoldingKey(key);
                    keyboard_motion += handle_keypress(key, pose);
                }
                ElementState::Released => {
                    self.keyboard_state = KeyboardState::Idle;
                }
            }
        }

        // If there were no keyboard inputs, but we're still holding down a key, act as if that key was pressed
        if let (KeyboardState::HoldingKey(key), true) =
            (&self.keyboard_state, keyboard_input.is_empty())
        {
            keyboard_motion = handle_keypress(*key, pose);
        }

        pose.position += keyboard_motion * movement_speed;
    }
}

fn handle_mouse_movement(movement: Vec2, pose: &mut Pose) {
    pose.yaw -= movement.x;
    const MIN_PITCH: f32 = -std::f32::consts::FRAC_PI_4;
    const MAX_PITCH: f32 = std::f32::consts::FRAC_PI_4;
    pose.pitch = (pose.pitch - movement.y).clamp(MIN_PITCH, MAX_PITCH);
}

fn handle_keypress(key: VirtualKeyCode, pose: &mut Pose) -> Vec3 {
    let orientation = pose.orientation();
    let mut position = Vec3::ZERO;
    // get the forward vector rotated by the camera rotation quaternion
    let forward = orientation * Vec3::NEG_Z;
    // get the right vector rotated by the camera rotation quaternion
    let right = orientation * Vec3::X;
    let up = Vec3::Y;

    match key {
        winit::event::VirtualKeyCode::W => {
            position.x += forward.x;
            position.y += forward.y;
            position.z += forward.z;
        }
        winit::event::VirtualKeyCode::S => {
            position.x -= forward.x;
            position.y -= forward.y;
            position.z -= forward.z;
        }
        winit::event::VirtualKeyCode::A => {
            position.x -= right.x;
            position.y -= right.y;
            position.z -= right.z;
        }
        winit::event::VirtualKeyCode::D => {
            position.x += right.x;
            position.y += right.y;
            position.z += right.z;
        }
        winit::event::VirtualKeyCode::Space => {
            position.y += up.y;
        }
        winit::event::VirtualKeyCode::LShift => {
            position.y -= up.y;
        }
        _ => {}
    }

    position
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
            pitch: 0.,
            yaw: 0.,
        };

        // press w
        input_context.update(1.0, &[press(VirtualKeyCode::W)], &[], &mut pose);
        assert_eq!(pose.position, [0., 0., -INPUT_SPEED].into());

        // keep holding it
        input_context.update(1.0, &[], &[], &mut pose);
        assert_eq!(pose.position, [0., 0., -INPUT_SPEED * 2.0].into());

        // release
        input_context.update(1.0, &[release(VirtualKeyCode::W)], &[], &mut pose);
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
