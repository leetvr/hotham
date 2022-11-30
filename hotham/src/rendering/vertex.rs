use ash::vk;
use glam::{Vec2, Vec3, Vec4};

/// Representation of a single vertex, usually imported from a glTF file.
#[repr(C)]
#[derive(Clone, Debug, Copy, PartialEq, Default)]
pub struct Vertex {
    // /// Position in model space
    // pub position: Vec3,
    /// Normal in model space
    pub normal: Vec3,
    /// First set of texture coordinates
    pub texture_coords: Vec2,
    /// Joint indices (for skinning), one byte per index.
    pub joint_indices: u32,
    /// Joint weights (for skinning), one byte per weight.
    pub joint_weights: u32,
}

impl Vertex {
    /// Create a new vertex
    pub fn new(normal: Vec3, texture_coords: Vec2, joint_indices: u32, joint_weights: u32) -> Self {
        Self {
            // position,
            normal,
            texture_coords,
            joint_indices,
            joint_weights,
        }
    }

    /// Create a new vertex from a zip - useful when importing from glTF
    // Clippy warning suppressed for adjudication separately
    #[cfg_attr(feature = "cargo-clippy", allow(clippy::type_complexity))]
    pub fn from_zip(t: (Vec3, Vec2, [u8; 4], Vec4)) -> Self {
        // Normalize weights to 0 <= w <= 255 while avoiding division with zero.
        let max_weight = t.3.max_element().max(f32::EPSILON);
        let weight_normalization = 255.0 / max_weight;
        Vertex::new(
            t.0,
            t.1,
            // Pack indices into one u32 with one byte per index.
            (t.2[0] as u32)
                + (t.2[1] as u32) * 256
                + (t.2[2] as u32) * 256 * 256
                + (t.2[3] as u32) * 256 * 256 * 256,
            // Pack weights into one u32 with one byte per weight.
            ((t.3[0] * weight_normalization).round() as u32)
                + ((t.3[1] * weight_normalization).round() as u32) * 256
                + ((t.3[2] * weight_normalization).round() as u32) * 256 * 256
                + ((t.3[3] * weight_normalization).round() as u32) * 256 * 256 * 256,
        )
    }
}

impl Vertex {
    /// Get the vertex attributes to be used in the Vertex Shader
    pub fn attribute_descriptions() -> Vec<vk::VertexInputAttributeDescription> {
        let position = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(0)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(0)
            .build();

        let normal = vk::VertexInputAttributeDescription::builder()
            .binding(1)
            .location(1)
            .format(vk::Format::R32G32B32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, normal) as _)
            .build();

        let texture_coords = vk::VertexInputAttributeDescription::builder()
            .binding(1)
            .location(2)
            .format(vk::Format::R32G32_SFLOAT)
            .offset(memoffset::offset_of!(Vertex, texture_coords) as _)
            .build();

        let joint_indices = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(3)
            .format(vk::Format::R32_UINT)
            .offset(memoffset::offset_of!(Vertex, joint_indices) as _)
            .build();

        let joint_weights = vk::VertexInputAttributeDescription::builder()
            .binding(0)
            .location(4)
            .format(vk::Format::R32_UINT)
            .offset(memoffset::offset_of!(Vertex, joint_weights) as _)
            .build();

        vec![
            position,
            normal,
            texture_coords,
            joint_indices,
            joint_weights,
        ]
    }
}
