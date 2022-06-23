use ash::vk;
use id_arena::Arena;
use nalgebra::{Matrix4, Vector3, Vector4};
use vulkan_context::VulkanContext;

use crate::{components::Mesh, resources::vulkan_context};

use super::{
    descriptors::{self, Descriptors},
    gambier_buffer::GambierBuffer as Buffer,
    image::Image,
    material::Material,
    texture::Texture,
    vertex::Vertex,
};

static VERTEX_BUFFER_SIZE: usize = 1_000_000; // TODO
static DRAW_DATA_BUFFER_SIZE: usize = 10_000; // TODO
static MATERIAL_BUFFER_SIZE: usize = 10_000; // TODO

/// A container that holds all of the resources required to draw a frame.
pub struct Resources {
    /// All the vertices that will be drawn this frame.
    pub vertex_buffer: Buffer<Vertex>,

    /// All the indices that will be drawn this frame.
    pub index_buffer: Buffer<u32>,

    /// Data for the primitives that will be drawn this frame, indexed by gl_DrawId
    pub draw_data_buffer: Buffer<DrawData>,

    /// Buffer for materials, indexed by material_id in DrawData
    pub materials_buffer: Buffer<Material>,

    /// The actual draw calls for this frame.
    pub draw_indirect_buffer: Buffer<vk::DrawIndexedIndirectCommand>,

    /// Mesh data used to generate DrawData
    pub mesh_data: Arena<Mesh>,

    /// Shared sampler in repeat mode, takes care of most things
    pub texture_sampler: vk::Sampler,

    /// Shared sampler
    pub cube_sampler: vk::Sampler,

    /// Texture descriptor information
    texture_count: u32,
}

impl Resources {
    /// Create all the buffers required and update the relevant descriptor sets.
    pub(crate) unsafe fn new(vulkan_context: &VulkanContext, descriptors: &Descriptors) -> Self {
        let vertex_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::VERTEX_BUFFER,
            VERTEX_BUFFER_SIZE,
        );

        let index_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::INDEX_BUFFER,
            VERTEX_BUFFER_SIZE,
        );

        let draw_data_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            DRAW_DATA_BUFFER_SIZE,
        );
        draw_data_buffer.update_descriptor_set(&vulkan_context.device, descriptors.set, 0);

        let materials_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MATERIAL_BUFFER_SIZE,
        );
        materials_buffer.update_descriptor_set(&vulkan_context.device, descriptors.set, 1);

        let draw_indirect_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MATERIAL_BUFFER_SIZE,
        );
        draw_indirect_buffer.update_descriptor_set(&vulkan_context.device, descriptors.set, 2);

        let texture_sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::REPEAT, 1)
            .unwrap();

        let cube_sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::CLAMP_TO_EDGE, 1)
            .unwrap();

        Self {
            vertex_buffer,
            index_buffer,
            draw_data_buffer,
            materials_buffer,
            draw_indirect_buffer,
            mesh_data: Default::default(),
            texture_count: 0,
            texture_sampler,
            cube_sampler,
        }
    }

    pub(crate) unsafe fn write_texture_to_array(
        &mut self,
        vulkan_context: &VulkanContext,
        descriptors: &Descriptors,
        image: &Image,
    ) -> u32 {
        let sampler = if image.format == vk::Format::R16G16_SFLOAT || image.layer_count == 6 {
            self.cube_sampler
        } else {
            self.texture_sampler
        };

        let index = self.texture_count;
        descriptors.write_texture_descriptor(vulkan_context, image.view, sampler, index);
        self.texture_count += 1;

        index
    }
}

/// Instructions on how to draw this primitive
pub struct DrawData {
    /// The transform of the parent mesh
    pub transform: Matrix4<f32>,
    /// A bounding sphere for the primitive in x, y, z, radius format
    pub bounding_sphere: Vector4<f32>,
    /// The ID of the material to use.
    pub material_id: u32,
    /// Padding
    _pad: Vector3<f32>,
}
