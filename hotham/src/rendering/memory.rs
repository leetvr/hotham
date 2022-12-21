use crate::contexts::VulkanContext;
use ash::vk;

pub(crate) unsafe fn allocate_memory(
    vulkan_context: &VulkanContext,
    memory_requirements: vk::MemoryRequirements,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> vk::DeviceMemory {
    let instance = &vulkan_context.instance;
    let device = &vulkan_context.device;
    let physical_device = vulkan_context.physical_device;

    let memory_type_bits_requirement = memory_requirements.memory_type_bits;
    let memory_properties = instance.get_physical_device_memory_properties(physical_device);
    let memory_type_index = find_memory_type_index(
        memory_properties,
        memory_type_bits_requirement,
        memory_property_flags,
    );
    println!("[HOTHAM_VULKAN] Using memory type {memory_type_index}");
    device
        .allocate_memory(
            &vk::MemoryAllocateInfo::builder()
                .allocation_size(memory_requirements.size)
                .memory_type_index(memory_type_index as _),
            None,
        )
        .unwrap()
}

fn find_memory_type_index(
    memory_properties: vk::PhysicalDeviceMemoryProperties,
    memory_type_bits_requirement: u32,
    memory_property_flags: vk::MemoryPropertyFlags,
) -> usize {
    let mut memory_type_index = !0;
    for memory_index in 0..memory_properties.memory_type_count as usize {
        let memory_type_bits: u32 = 1 << memory_index;
        let is_required_memory_type = (memory_type_bits_requirement & memory_type_bits) != 0;
        let properties = memory_properties.memory_types[memory_index].property_flags;

        if is_required_memory_type && properties.contains(memory_property_flags) {
            memory_type_index = memory_index;
            break;
        }
    }
    if memory_type_index == !0 {
        panic!("Unable to find suitable memory!")
    }
    memory_type_index
}
