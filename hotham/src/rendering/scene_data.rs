// TODO: Should these be components?
use nalgebra::{vector, Matrix4, Vector3, Vector4};
use serde::{Deserialize, Serialize};

/// Data about the current scene. Sent to the vertex and fragment shaders
#[derive(Deserialize, Serialize, Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneData {
    /// Projection matrices (one per eye)
    pub projection: [Matrix4<f32>; 2],
    /// View matrices (one per eye)
    pub view: [Matrix4<f32>; 2],
    /// Position of the cameras (one per eye)
    pub camera_position: [Vector4<f32>; 2],
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            view: [Matrix4::identity(), Matrix4::identity()],
            projection: [Matrix4::identity(), Matrix4::identity()],
            camera_position: [Vector4::zeros(), Vector4::zeros()],
        }
    }
}

/// Parameters sent to the fragment shader to tweak the scene
/// See `pbr.frag` for more information
#[derive(Deserialize, Serialize, Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneParams {
    /// Direction of the global light
    pub light_direction: Vector4<f32>,
    /// Level of exposure
    pub exposure: f32,
    /// Gamma
    pub gamma: f32,
    /// Prefiltered Cube MIP Levels
    pub prefiltered_cube_mip_levels: f32,
    /// How much should the IBL ambient light be scaled?
    pub scale_ibl_ambient: f32,
    /// Debug view inputs (see pbr.frag)
    pub debug_view_inputs: f32,
    /// Debug view equation (see pbr.frag)
    pub debug_view_equation: f32,
}

impl Default for SceneParams {
    fn default() -> Self {
        let light_source: Vector3<f32> =
            vector![75_f32.to_radians(), 40_f32.to_radians(), 0_f32.to_radians()];
        let x = light_source.x.sin() * light_source.y.cos();
        let y = light_source.y.sin();
        let z = light_source.x.cos() * light_source.y.cos();

        let light_direction = vector![x, y, z, 0.];
        SceneParams {
            light_direction,
            exposure: 4.5,
            gamma: 2.2,
            prefiltered_cube_mip_levels: 10.,
            scale_ibl_ambient: 0.1,
            debug_view_inputs: 0.,
            debug_view_equation: 0.,
        }
    }
}
