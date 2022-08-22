use ash::vk;
use id_arena::Arena;
use nalgebra::{Matrix4, Vector4};
use vulkan_context::VulkanContext;

use crate::resources::vulkan_context;

use super::{
    buffer::Buffer,
    descriptors::{Descriptors, MATERIALS_BINDING, SKINS_BINDING},
    image::Image,
    material::Material,
    mesh_data::MeshData,
    texture::{parse_ktx2, DEFAULT_COMPONENT_MAPPING},
    vertex::Vertex,
};

static VERTEX_BUFFER_SIZE: usize = 2_000_000; // TODO
static MATERIAL_BUFFER_SIZE: usize = 10_000; // TODO
static SKINS_BUFFER_SIZE: usize = 100; // TODO

pub(crate) const MAX_JOINTS: usize = 64;

/// A container that holds all of the resources required to draw a frame.
pub struct Resources {
    /// All the vertices that will be drawn this frame.
    pub vertex_buffer: Buffer<Vertex>,

    /// All the indices that will be drawn this frame.
    pub index_buffer: Buffer<u32>,

    /// Buffer for materials, indexed by material_id in DrawData
    pub materials_buffer: Buffer<Material>,

    /// Mesh data used to generate DrawData
    pub mesh_data: Arena<MeshData>,

    /// Buffer for skins
    pub skins_buffer: Buffer<[Matrix4<f32>; 64]>,

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

        let mut materials_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            MATERIAL_BUFFER_SIZE,
        );
        for set in descriptors.sets {
            materials_buffer.update_descriptor_set(&vulkan_context.device, set, MATERIALS_BINDING);
        }

        // RESERVE index 0 for the default material, available as the material::NO_MATERIAL constant.
        materials_buffer.push(&Material::default());

        let skins_buffer = Buffer::new(
            vulkan_context,
            vk::BufferUsageFlags::STORAGE_BUFFER,
            SKINS_BUFFER_SIZE,
        );

        for set in descriptors.sets {
            skins_buffer.update_descriptor_set(&vulkan_context.device, set, SKINS_BINDING);
        }

        let texture_sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::REPEAT)
            .unwrap();

        let cube_sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::CLAMP_TO_EDGE)
            .unwrap();

        load_ibl_textures(vulkan_context, descriptors, cube_sampler);

        Self {
            vertex_buffer,
            index_buffer,
            materials_buffer,
            skins_buffer,
            mesh_data: Default::default(),
            texture_count: 1, // IMPORTANT! Because we stashed the BRDF Lut texture in here, make sure we increment the count accordingly
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
        // There doesn't seem any reason to add support for dynamic cube maps yet as there isn't any user facing way of loading them.
        let sampler = self.texture_sampler;

        let index = self.texture_count;
        descriptors.write_texture_descriptor(vulkan_context, image.view, sampler, index);
        self.texture_count += 1;

        index
    }
}

// Upload the textures required for Image Based Lighting. A bit of silliness is required here.
// Our normal methods of creating textures are somewhat limited here as we don't have access to RenderContext.
// A better way to handle this would be to make Texture a little more flexible, but we can get to that.
fn load_ibl_textures(
    vulkan_context: &VulkanContext,
    descriptors: &Descriptors,
    cube_texture_sampler: vk::Sampler,
) {
    // First, load in the LUT file.
    let brdf_lut_file = include_bytes!("../../data/brdf_lut.ktx2");
    let ktx2_image = parse_ktx2(brdf_lut_file);

    let image = vulkan_context
        .create_image_with_component_mapping(
            ktx2_image.format,
            &ktx2_image.extent,
            vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
            1,
            1,
            DEFAULT_COMPONENT_MAPPING,
        )
        .unwrap();

    vulkan_context.upload_image(&ktx2_image.image_buf, 1, vec![0], &image);
    let texture_sampler = vulkan_context
        .create_texture_sampler(vk::SamplerAddressMode::CLAMP_TO_EDGE)
        .unwrap();

    unsafe {
        descriptors.write_texture_descriptor(vulkan_context, image.view, texture_sampler, 0);
    }

    // OK. Next we've got to load in the cubemaps.
    let cubemaps = [
        include_bytes!("../../data/environment_map_diffuse.ktx2").to_vec(),
        include_bytes!("../../data/environment_map_specular.ktx2").to_vec(),
    ];

    for (index, image) in cubemaps.iter().enumerate() {
        let ktx2_image = parse_ktx2(image);
        let mip_levels = ktx2_image.mip_levels;

        // Right. Now we've got to do the array/mip count dance.
        let image = vulkan_context
            .create_image_with_component_mapping(
                ktx2_image.format,
                &ktx2_image.extent,
                vk::ImageUsageFlags::SAMPLED | vk::ImageUsageFlags::TRANSFER_DST,
                6,
                mip_levels,
                DEFAULT_COMPONENT_MAPPING,
            )
            .unwrap();

        vulkan_context.upload_image(
            &ktx2_image.image_buf,
            mip_levels,
            ktx2_image.offsets,
            &image,
        );

        unsafe {
            descriptors.write_cube_texture_descriptor(
                vulkan_context,
                image.view,
                cube_texture_sampler,
                index as _,
            );
        }
    }
}

/// Instructions on how to draw this primitive
#[derive(Debug, Default, Clone)]
#[repr(C, align(16))]
pub struct DrawData {
    /// The transform of the parent mesh
    pub global_from_local: Matrix4<f32>,
    /// The inverse of the transform of the parent mesh
    /// Transform normals by multiplying with the matrix on the right hand side
    pub local_from_global: Matrix4<f32>,
    /// The ID of the material to use.
    pub material_id: u32,
    /// An optional skin to use.
    pub skin_id: u32,
}

/// Information for the culling shader on how to cull this primitive.
#[derive(Debug, Default, Clone)]
#[repr(C, align(16))]
pub(crate) struct PrimitiveCullData {
    pub bounding_sphere: Vector4<f32>,
    pub index_instance: u32,
    pub index_offset: u32,
    pub visible: bool,
}
