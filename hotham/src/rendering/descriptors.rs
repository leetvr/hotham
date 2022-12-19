use std::convert::TryInto;

use crate::contexts::{render_context::PIPELINE_DEPTH, VulkanContext};
use ash::vk;

pub const DRAW_DATA_BINDING: u32 = 0;
pub const SKINS_BINDING: u32 = 1;
pub const SCENE_DATA_BINDING: u32 = 2;
pub const TEXTURE_BINDING: u32 = 3;
pub const CUBE_TEXTURE_BINDING: u32 = 4;

pub const PRIMITIVE_CULL_DATA_BINDING: u32 = 0;
pub const CULL_PARAMS_BINDING: u32 = 1;

const TEXTURE_BINDING_DESCRIPTOR_COUNT: u32 = 10_000;

/// A wrapper around all the various bits of descriptor functionality
#[derive(Clone, Debug)]
pub struct Descriptors {
    pub graphics_layout: vk::DescriptorSetLayout,
    pub compute_layout: vk::DescriptorSetLayout,
    pub texture_layout: vk::DescriptorSetLayout,
    // One descriptor set per frame
    pub sets: [vk::DescriptorSet; PIPELINE_DEPTH],
    // One descriptor set per frame
    pub compute_sets: [vk::DescriptorSet; PIPELINE_DEPTH],
    #[allow(unused)]
    pub pool: vk::DescriptorPool,
}

impl Descriptors {
    pub unsafe fn new(vulkan_context: &VulkanContext) -> Self {
        // First, create a pool.
        let pool = create_descriptor_pool(&vulkan_context.device);

        // Then create a layout.
        let (graphics_layout, compute_layout, texture_layout) =
            create_descriptor_layouts(&vulkan_context.device);

        // Finally, allocate the shared descriptor set.
        let sets = allocate_descriptor_sets(vulkan_context, pool, graphics_layout);
        let compute_sets = allocate_compute_descriptor_sets(vulkan_context, pool, compute_layout);

        Self {
            graphics_layout,
            sets,
            pool,
            compute_layout,
            compute_sets,
            texture_layout,
        }
    }

    pub unsafe fn write_texture_descriptor(
        &self,
        vulkan_context: &VulkanContext,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        array_index: u32,
    ) {
        let image_info = vk::DescriptorImageInfo {
            sampler,
            image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let texture_writes = self.sets.map(|set| {
            vk::WriteDescriptorSet::builder()
                .image_info(std::slice::from_ref(&image_info))
                .dst_binding(TEXTURE_BINDING)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_array_element(array_index)
                .dst_set(set)
                .build()
        });

        vulkan_context
            .device
            .update_descriptor_sets(&texture_writes, &[]);
    }

    pub unsafe fn write_cube_texture_descriptor(
        &self,
        vulkan_context: &VulkanContext,
        image_view: vk::ImageView,
        sampler: vk::Sampler,
        array_index: u32,
    ) {
        let image_info = vk::DescriptorImageInfo {
            sampler,
            image_view,
            image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
        };

        let texture_writes = self.sets.map(|set| {
            vk::WriteDescriptorSet::builder()
                .image_info(std::slice::from_ref(&image_info))
                .dst_binding(CUBE_TEXTURE_BINDING)
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_array_element(array_index)
                .dst_set(set)
                .build()
        });

        vulkan_context
            .device
            .update_descriptor_sets(&texture_writes, &[]);
    }
}

unsafe fn allocate_descriptor_sets(
    vulkan_context: &VulkanContext,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> [vk::DescriptorSet; PIPELINE_DEPTH] {
    let layouts = [layout; PIPELINE_DEPTH];

    vulkan_context
        .device
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts),
        )
        .unwrap()
        .as_slice()
        .try_into()
        .unwrap()
}

unsafe fn allocate_compute_descriptor_sets(
    vulkan_context: &VulkanContext,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> [vk::DescriptorSet; PIPELINE_DEPTH] {
    let layouts = [layout; PIPELINE_DEPTH];

    vulkan_context
        .device
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts),
        )
        .unwrap()
        .as_slice()
        .try_into()
        .unwrap()
}

unsafe fn create_descriptor_layouts(
    device: &ash::Device,
) -> (
    vk::DescriptorSetLayout,
    vk::DescriptorSetLayout,
    vk::DescriptorSetLayout,
) {
    let graphics_bindings = [
        // Draw Data
        vk::DescriptorSetLayoutBinding {
            binding: DRAW_DATA_BINDING,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            descriptor_count: 1,
            ..Default::default()
        },
        // Skins
        vk::DescriptorSetLayoutBinding {
            binding: SKINS_BINDING,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            descriptor_count: 1,
            ..Default::default()
        },
        // Scene Data
        vk::DescriptorSetLayoutBinding {
            binding: SCENE_DATA_BINDING,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1,
            ..Default::default()
        },
        // Textures
        vk::DescriptorSetLayoutBinding {
            binding: TEXTURE_BINDING,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: TEXTURE_BINDING_DESCRIPTOR_COUNT,
            ..Default::default()
        },
        // Cube Textures
        vk::DescriptorSetLayoutBinding {
            binding: CUBE_TEXTURE_BINDING,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 2,
            ..Default::default()
        },
    ];

    let compute_bindings = [
        // Primitive Cull Data
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Cull Params
        vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
    ];

    let texture_bindings = [
        // Textures
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: TEXTURE_BINDING_DESCRIPTOR_COUNT,
            ..Default::default()
        },
    ];

    let flags = vk::DescriptorBindingFlags::PARTIALLY_BOUND;

    let descriptor_flags = [
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        flags,
        vk::DescriptorBindingFlags::empty(),
    ];
    let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::builder()
        .binding_flags(&descriptor_flags);

    let graphics_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&graphics_bindings)
                .push_next(&mut binding_flags),
            None,
        )
        .unwrap();

    let compute_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&compute_bindings),
            None,
        )
        .unwrap();

    let texture_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&texture_bindings),
            None,
        )
        .unwrap();

    (graphics_layout, compute_layout, texture_layout)
}

unsafe fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
    let pool_sizes = [
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 100,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::UNIFORM_BUFFER,
            descriptor_count: 100,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 60_000,
        },
    ];
    device
        .create_descriptor_pool(
            &vk::DescriptorPoolCreateInfo::builder()
                .pool_sizes(&pool_sizes)
                .max_sets(1000),
            None,
        )
        .unwrap()
}
