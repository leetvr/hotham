use ash::vk;

use crate::resources::VulkanContext;

/// A wrapper around all the various bits of descriptor functionality
#[derive(Clone, Debug)]
pub(crate) struct Descriptors {
    pub layout: vk::DescriptorSetLayout,
    pub set: vk::DescriptorSet,
    pub pool: vk::DescriptorPool,
}

impl Descriptors {
    pub unsafe fn new(vulkan_context: &VulkanContext) -> Self {
        // First, create a pool.
        let pool = create_descriptor_pool(&vulkan_context.device);

        // Then create a layout.
        let layout = create_descriptor_layouts(&vulkan_context.device);

        // Finally, allocate the shared descriptor set.
        let set = allocate_descriptor_set(vulkan_context, pool, layout);

        Self { layout, set, pool }
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

        let texture_write = vk::WriteDescriptorSet::builder()
            .image_info(std::slice::from_ref(&image_info))
            .dst_binding(3)
            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
            .dst_array_element(array_index)
            .dst_set(self.set);

        vulkan_context
            .device
            .update_descriptor_sets(std::slice::from_ref(&texture_write), &[]);
    }
}

unsafe fn allocate_descriptor_set(
    vulkan_context: &VulkanContext,
    pool: vk::DescriptorPool,
    layout: vk::DescriptorSetLayout,
) -> vk::DescriptorSet {
    let mut descriptor_counts =
        vk::DescriptorSetVariableDescriptorCountAllocateInfo::builder().descriptor_counts(&[1000]);
    let set = vulkan_context
        .device
        .allocate_descriptor_sets(
            &vk::DescriptorSetAllocateInfo::builder()
                .descriptor_pool(pool)
                .set_layouts(std::slice::from_ref(&layout))
                .push_next(&mut descriptor_counts),
        )
        .unwrap()[0];
    set
}

unsafe fn create_descriptor_layouts(device: &ash::Device) -> vk::DescriptorSetLayout {
    let bindings = [
        // Draw Data
        vk::DescriptorSetLayoutBinding {
            binding: 0,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::COMPUTE,
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
        // Draw Indirect Buffer
        vk::DescriptorSetLayoutBinding {
            binding: 2,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::COMPUTE,
            descriptor_count: 1,
            ..Default::default()
        },
        // Textures
        vk::DescriptorSetLayoutBinding {
            binding: 3,
            descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            stage_flags: vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1000,
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
        flags,
    ];
    let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::builder()
        .binding_flags(&descriptor_flags);

    let shared_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&bindings)
                .push_next(&mut binding_flags)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL),
            None,
        )
        .unwrap();

    shared_layout
}

unsafe fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
    let pool_sizes = [
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::STORAGE_BUFFER,
            descriptor_count: 100,
        },
        vk::DescriptorPoolSize {
            ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
            descriptor_count: 10_000,
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
