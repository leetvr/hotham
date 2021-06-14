use ash::vk;
use cgmath::{vec3, Matrix4, Vector3};
use rand::random;

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Vertex {
    pub position: Vector3<f32>,
    pub color: Vector3<f32>,
}

impl Vertex {
    pub fn new(position: Vector3<f32>, color: Vector3<f32>, _transform: Matrix4<f32>) -> Self {
        Self { position, color }
    }

    pub fn pos(position: Vector3<f32>) -> Self {
        let color = vec3(random(), random(), random());
        let _transform = Matrix4::from_scale(1.0);
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
