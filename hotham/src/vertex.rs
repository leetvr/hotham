use ash::vk;
use cgmath::{Vector2, Vector3, Vector4};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub texture_coords: Vector2<f32>,
    pub normal: Vector3<f32>,
    pub tangent: Vector4<f32>,
}

impl Vertex {
    pub fn new(
        position: Vector3<f32>,
        texture_coords: Vector2<f32>,
        normal: Vector3<f32>,
        tangent: Vector4<f32>,
    ) -> Self {
        Self {
            position,
            texture_coords,
            normal,
            tangent,
        }
    }

    pub fn from_zip(t: (Vector3<f32>, Vector2<f32>, Vector3<f32>, Vector4<f32>)) -> Self {
        Vertex::new(t.0, t.1, t.2, t.3)
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

        let texture_coords = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords) as _)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, normal) as _)
            .build();

        let tangent = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, tangent) as _)
            .build();

        vec![position, texture_coords, normal, tangent]
    }
}
