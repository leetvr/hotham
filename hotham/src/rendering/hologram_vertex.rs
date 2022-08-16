use ash::vk;
use nalgebra::Vector3;

/// Representation of a single vertex, usually imported from a glTF file.
#[repr(C)]
#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub struct HologramVertex {
    /// Position in model space
    pub position: Vector3<f32>,
}

impl HologramVertex {
    /// Create a new vertex
    pub fn new(position: Vector3<f32>) -> Self {
        Self { position }
    }
}

impl HologramVertex {
    /// Get the vertex attributes to be used in the QuadricVertex Shader
    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(HologramVertex, position) as _)
            .build();

        vec![position]
    }
}
