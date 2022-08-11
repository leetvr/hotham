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
    /// Scene Parameters - x = IBL intensity, y = unused, z = debug render inputs, w = debug render algorithm
    pub params: Vector4<f32>,
}

impl Default for SceneData {
    fn default() -> Self {
        // TODO: Pick a reasonable default.
        let light_direction = new_directional_light(-27., 67., -35.);
        Self {
            view_projection: [Matrix4::identity(), Matrix4::identity()],
            camera_position: [Vector4::zeros(), Vector4::zeros()],
            light_direction,
            params: [1., 0., 0., 0.].into(),
        }
    }
}

/// Create a new directional light
pub fn new_directional_light(x: f32, y: f32, z: f32) -> Vector4<f32> {
    [x.to_radians(), y.to_radians(), z.to_radians(), 1.].into()
}
