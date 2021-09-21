// TODO: Should these be components?
use nalgebra::{vector, Matrix4, Vector3, Vector4};

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneData {
    pub projection: [Matrix4<f32>; 2],
    pub view: [Matrix4<f32>; 2],
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

#[derive(Clone, Debug, Copy)]
#[repr(C)]
pub struct SceneParams {
    pub light_direction: Vector4<f32>,
    pub exposure: f32,
    pub gamma: f32,
    pub prefiltered_cube_mip_levels: f32,
    pub scale_ibl_ambient: f32,
    pub debug_view_inputs: f32,
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
