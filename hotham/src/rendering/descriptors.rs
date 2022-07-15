use std::convert::TryInto;

use crate::resources::{render_context::PIELINE_DEPTH, VulkanContext};
use ash::vk;

static TEXTURE_BINDING: u32 = 4;

/// A wrapper around all the various bits of descriptor functionality
#[derive(Clone, Debug)]
pub(crate) struct Descriptors {
    pub graphics_layout: vk::DescriptorSetLayout,
    pub compute_layout: vk::DescriptorSetLayout,
    // One descriptor set per frame
    pub sets: [vk::DescriptorSet; PIELINE_DEPTH],
    // One descriptor set per frame
    pub compute_sets: [vk::DescriptorSet; PIELINE_DEPTH],
    #[allow(unused)]
    pub pool: vk::DescriptorPool,
}

impl Descriptors {
    pub unsafe fn new(vulkan_context: &VulkanContext) -> Self {
        // First, create a pool.
        let pool = create_descriptor_pool(&vulkan_context.device);

        // Then create a layout.
        let (graphics_layout, compute_layout) = create_descriptor_layouts(&vulkan_context.device);

        // Finally, allocate the shared descriptor set.
        let sets = allocate_descriptor_sets(vulkan_context, pool, graphics_layout);
        let compute_sets = allocate_compute_descriptor_sets(vulkan_context, pool, compute_layout);

        Self {
            graphics_layout,
            sets,
            pool,
            compute_layout,
            compute_sets,
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
}

unsafe fn allocate_descriptor_sets(
    vulkan_context: &VulkanContext,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> [vk::DescriptorSet; PIELINE_DEPTH] {
    let mut descriptor_counts = vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder()
        .descriptor_counts(&[10_000, 10_000, 10_000]);
    let layouts = [layout; PIELINE_DEPTH];

    vulkan_context
        .device
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(&layouts)
                .push_next(&mut descriptor_counts),
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
) -> [vk::DescriptorSet; PIELINE_DEPTH] {
    let layouts = [layout; PIELINE_DEPTH];

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
) -> (vk::DescriptorSetLayout, vk::DescriptorSetLayout) {
    let graphics_bindings = [
        // Draw Data
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            descriptor_count: 1,
            ..Default::default()
        },
        // Materials
        vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1,
            ..Default::default()
        },
        // Skins
        vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX,
            descriptor_count: 1,
            ..Default::default()
        },
        // Scene Data
        vk::DescriptorSetLayoutBinding {
            binding: 3,
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
            descriptor_count: 10_000,
            ..Default::default()
        },
    ];

    let compute_bindings = [
        // Draw Data
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Draw Indirect Buffer
        vk::DescriptorSetLayoutBinding {
            binding: 1,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Cull Data
        vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::UNIFORM_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
    ];

    let flags = vk::DescriptorBindingFlags::PARTIALLY_BOUND
        | vk::DescriptorBindingFlags::VARIABLE_DESCRIPTOR_COUNT
        | vk::DescriptorBindingFlags::UPDATE_AFTER_BIND;

    let descriptor_flags = [
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        vk::DescriptorBindingFlags::empty(),
        flags,
    ];
    let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::builder()
        .binding_flags(&descriptor_flags);

    let graphics_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&graphics_bindings)
                .push_next(&mut binding_flags)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL),
            None,
        )
        .unwrap();

    let compute_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&compute_bindings),
            None,
        )
        .unwrap();

    (graphics_layout, compute_layout)
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
                .max_sets(1000)
                .flags(vk::DescriptorPoolCreateFlags::UPDATE_AFTER_BIND),
            None,
        )
        .unwrap()
}
