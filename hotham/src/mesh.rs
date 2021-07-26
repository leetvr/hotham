use ash::vk;
use cgmath::{vec2, vec3, vec4, Matrix4};

#[derive(Debug, Clone)]
pub struct Mesh {
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub index_buffer: vk::Buffer,
    pub vertex_buffer: vk::Buffer,
    pub num_indices: u32,
    pub transform: Matrix4<f32>,
}
