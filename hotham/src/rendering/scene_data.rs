use nalgebra::{Matrix4, Vector4};
use serde::{Deserialize, Serialize};

/// Data about the current scene. Sent to the vertex and fragment shaders
#[derive(Deserialize, Serialize, Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneData {
    /// View-Projection matrices (one per eye)
    pub view_projection: [Matrix4<f32>; 2],
    /// Position of the cameras (one per eye)
    pub camera_position: [Vector4<f32>; 2],
    /// Direction of global light
    pub light_direction: Vector4<f32>,
    /// Debug information - x = debug inputs, y = debug algorithm, z = unused
    pub debug_data: Vector4<f32>,
}

impl Default for SceneData {
    fn default() -> Self {
        let light_direction = new_directional_light(75_f32.to_radians(), 40_f32.to_radians());
        Self {
            view_projection: [Matrix4::identity(), Matrix4::identity()],
            camera_position: [Vector4::zeros(), Vector4::zeros()],
            light_direction,
            debug_data: Default::default(),
        }
    }
}

/// Create a new directional light
pub fn new_directional_light(x: f32, y: f32) -> Vector4<f32> {
    let x = x.sin() * y.cos();
    let y = y.sin();
    let z = x.cos() * y.cos();
    [x, y, z, 1.].into()
}
