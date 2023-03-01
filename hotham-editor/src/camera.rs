use std::time::Instant;

use glam::{Quat, Vec3};
use winit::event::KeyboardInput;

use crate::{input_context::InputContext, MouseInput};

#[derive(Debug, Clone)]
pub struct Camera {
    pose: Pose,
    input_context: InputContext,
}

impl Default for Camera {
    fn default() -> Self {
        let pose = Pose {
            position: [0., 1.4, 0.].into(),
            pitch: 0.,
            yaw: 0.,
        };
        Self {
            pose,
            input_context: Default::default(),
        }
    }
}

impl Camera {
    pub fn as_pose(&self) -> openxr_sys::Posef {
        (&self.pose).into()
    }

    pub fn process_input(
        &mut self,
        last_frame_time: Instant,
        keyboard_input: &[KeyboardInput],
        mouse_input: &[MouseInput],
    ) {
        let delta_time = (Instant::now() - last_frame_time).as_secs_f32();
        self.input_context
            .update(delta_time, keyboard_input, mouse_input, &mut self.pose)
    }
}

#[derive(Debug, Clone)]
pub struct Pose {
    pub position: Vec3,
    pub pitch: f32,
    pub yaw: f32,
}

impl Pose {
    pub fn orientation(&self) -> Quat {
        Quat::from_euler(glam::EulerRot::YXZ, self.yaw, self.pitch, 0.)
    }
}

impl From<&Pose> for openxr_sys::Posef {
    fn from(pose: &Pose) -> Self {
        let p = pose.position;
        let o = pose.orientation();

        openxr_sys::Posef {
            orientation: openxr_sys::Quaternionf {
                x: o.x,
                y: o.y,
                z: o.z,
                w: o.w,
            },
            position: openxr_sys::Vector3f {
                x: p.x,
                y: p.y,
                z: p.z,
            },
        }
    }
}
