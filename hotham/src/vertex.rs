use ash::vk;
use cgmath::{vec2, vec3, Vector2, Vector3};
use rand::random;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Vector3<f32>,
    pub texture_coords: Vector2<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>, texture_coords: Vector2<f32>) -> Self {
        Self {
            position,
            color,
            texture_coords,
        }
    }

    pub fn pos(position: Vector3<f32>) -> Self {
        let color = vec3(random(), random(), random());
        Self {
            position,
            color,
            texture_coords: vec2(0.0, 0.0),
        }
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

        let colour = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, color) as _)
            .build();

        let texture_coords = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords) as _)
            .build();

        vec![position, colour, texture_coords]
    }
}
