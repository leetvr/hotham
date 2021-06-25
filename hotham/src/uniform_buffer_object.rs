use cgmath::{Matrix4, SquareMatrix};

#[derive(Clone, Debug)]
pub struct UniformBufferObject {
    pub mvp: [Matrix4<f32>; 2],
    pub delta_time: f32,
}

impl Default for UniformBufferObject {
    fn default() -> Self {
        Self {
            mvp: [Matrix4::identity(), Matrix4::identity()],
            delta_time: 0.0,
        }
    }
}
