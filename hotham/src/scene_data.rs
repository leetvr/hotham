use cgmath::{vec4, Matrix4, SquareMatrix, Vector4};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct SceneData {
    pub view: [Matrix4<f32>; 2],
    pub projection: [Matrix4<f32>; 2],
    pub light_pos: Vector4<f32>,
}

impl Default for SceneData {
    fn default() -> Self {
        Self {
            view: [Matrix4::identity(), Matrix4::identity()],
            projection: [Matrix4::identity(), Matrix4::identity()],
            light_pos: vec4(0.0, 0.0, 0.0, 0.0),
        }
    }
}
