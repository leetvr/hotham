use crate::buffer::Buffer;
use cgmath::Matrix4;

#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    pub joint_matrices: Vec<Matrix4<f32>>,
    pub(crate) buffer: Buffer<Matrix4<f32>>,
}
