use ash::vk;
use nalgebra::{Vector2, Vector3, Vector4};

/// Representation of a single vertex, usually imported from a glTF file.
#[repr(C)]
#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub struct Vertex {
    /// Position in model space
    pub position: Vector3<f32>,
    /// Normal in model space
    pub normal: Vector3<f32>,
    /// Tangent
    pub tangent: Vector4<f32>,
    /// First set of texture coordinates
    pub texture_coords: Vector2<f32>,
    /// Joint indices (for skinning)
    pub joint_indices: Vector4<f32>,
    /// Joint weights (for skinning)
    pub joint_weights: Vector4<f32>,
}

impl Vertex {
    /// Create a new vertex
    pub fn new(
        position: Vector3<f32>,
        normal: Vector3<f32>,
        tangent: Vector4<f32>,
        texture_coords: Vector2<f32>,
        joint_indices: Vector4<f32>,
        joint_weights: Vector4<f32>,
    ) -> Self {
        Self {
            position,
            normal,
            tangent,
            texture_coords,
            joint_indices,
            joint_weights,
        }
    }

    /// Create a new vertex from a zip - useful when importing from glTF
    // Clippy warning suppressed for adjudication separately
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
    pub fn from_zip(
        t: (
            Vector3<f32>,
            Vector3<f32>,
            Vector4<f32>,
            Vector2<f32>,
            Vector4<f32>,
            Vector4<f32>,
        ),
    ) -> Self {
        Vertex::new(t.0, t.1, t.2, t.3, t.4, t.5)
    }
}

impl Vertex {
    /// Get the vertex attributes to be used in the Vertex Shader
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

        let tangent = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(2)
            .format(vk::Format::R32G32B32A32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, tangent) as _)
            .build();

        let texture_coords = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords) as _)
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
            tangent,
            texture_coords,
            joint_indices,
            joint_weights,
        ]
    }
}
