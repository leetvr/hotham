use cgmath::{vec4, Matrix4, SquareMatrix, Vector4};

#[derive(Clone, Debug)]
#[repr(C)]
pub struct UniformBufferObject {
    pub view: [Matrix4<f32>; 2],
    pub projection: [Matrix4<f32>; 2],
    pub delta_time: f32,
    pub light_pos: Vector4<f32>,
}

impl Default for UniformBufferObject {
    fn default() -> Self {
        Self {
            view: [Matrix4::identity(), Matrix4::identity()],
            projection: [Matrix4::identity(), Matrix4::identity()],
            delta_time: 0.0,
            light_pos: vec4(0.0, 0.0, 0.0, 0.0),
        }
    }
}
