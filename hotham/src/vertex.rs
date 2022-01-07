use ash::vk;
use nalgebra::{Vector2, Vector3, Vector4};

#[repr(C)]
#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub normal: Vector3<f32>,
    pub texture_coords_0: Vector2<f32>,
    pub texture_coords_1: Vector2<f32>,
    pub joint_indices: Vector4<f32>,
    pub joint_weights: Vector4<f32>,
}

impl Vertex {
    pub fn new(
        position: Vector3<f32>,
        normal: Vector3<f32>,
        texture_coords_0: Vector2<f32>,
        texture_coords_1: Vector2<f32>,
        joint_indices: Vector4<f32>,
        joint_weights: Vector4<f32>,
    ) -> Self {
        Self {
            position,
            texture_coords_0,
            texture_coords_1,
            normal,
            joint_indices,
            joint_weights,
        }
    }

    pub fn from_zip(
        t: (
            Vector3<f32>,
            Vector3<f32>,
            Vector2<f32>,
            Vector2<f32>,
            Vector4<f32>,
            Vector4<f32>,
        ),
    ) -> Self {
        Vertex::new(t.0, t.1, t.2, t.3, t.4, t.5)
    }
}

impl Vertex {
    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, position) as _)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, normal) as _)
            .build();

        let texture_coords_0 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords_0) as _)
            .build();

        let texture_coords_1 = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords_1) as _)
            .build();

        let joint_indices = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(4)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, joint_indices) as _)
            .build();

        let joint_weights = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(5)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, joint_weights) as _)
            .build();

        vec![
            position,
            normal,
            texture_coords_0,
            texture_coords_1,
            joint_indices,
            joint_weights,
        ]
    }
}
