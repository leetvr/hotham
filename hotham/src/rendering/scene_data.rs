use glam::{Mat4, Vec4};
use serde::{Deserialize, Serialize};

use super::light::{Light, MAX_LIGHTS};

/// The amount of Image Based Lighting (IBL) to show in the scene
pub const DEFAULT_IBL_INTENSITY: f32 = 1.0;

/// Data about the current scene. Sent to the vertex and fragment shaders
#[derive(Deserialize, Serialize, Clone, Debug)]
#[repr(C)]
pub struct SceneData {
    /// View-Projection matrices (one per eye)
    pub view_projection: [Mat4; 2],
    /// Position of the cameras (one per eye)
    pub camera_position: [Vec4; 2],
    /// Scene Parameters - x = IBL intensity, y = unused, z = debug render inputs, w = debug render algorithm
    pub params: Vec4,
    /// Dynamic punctual lights
    pub lights: [Light; MAX_LIGHTS],
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            view_projection: [Mat4::IDENTITY, Mat4::IDENTITY],
            camera_position: [Vec4::ZERO, Vec4::ZERO],
            params: [DEFAULT_IBL_INTENSITY, 0., 0., 0.].into(),
            lights: [Light::none(), Light::none(), Light::none(), Light::none()],
        }
    }
}
