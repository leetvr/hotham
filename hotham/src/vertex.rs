use ash::vk;
use cgmath::Vector3;

#[derive(Clone, Debug)]
pub struct Vertex {
    position: Vector3<f32>,
    color: Vector3<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>) -> Self {
        Self { position, color }
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

        vec![position, colour]
    }
}
