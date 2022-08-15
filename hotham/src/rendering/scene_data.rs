use nalgebra::{Matrix4, Vector4};
use serde::{Deserialize, Serialize};

use super::light::{Light, MAX_LIGHTS};

/// The amount of Image Based Lighting (IBL) to show in the scene
pub const DEFAULT_IBL_INTENSITY: f32 = 0.4;

/// Data about the current scene. Sent to the vertex and fragment shaders
#[derive(Deserialize, Serialize, Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneData {
    /// View-Projection matrices (one per eye)
    pub view_projection: [Matrix4<f32>; 2],
    /// Position of the cameras (one per eye)
    pub camera_position: [Vector4<f32>; 2],
    /// Scene Parameters - x = IBL intensity, y = unused, z = debug render inputs, w = debug render algorithm
    pub params: Vector4<f32>,
    /// Dynamic punctual lights
    pub lights: [Light; MAX_LIGHTS],
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            view_projection: [Matrix4::identity(), Matrix4::identity()],
            camera_position: [Vector4::zeros(), Vector4::zeros()],
            params: [DEFAULT_IBL_INTENSITY, 0., 0., 0.].into(),
            lights: [Light::none(); MAX_LIGHTS],
        }
    }
}
