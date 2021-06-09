use anyhow::Result;
use ash::{version::DeviceV1_0, vk};

use crate::vulkan_context::VulkanContext;

pub(crate) struct Buffer {
    pub handle: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub item_count: usize,
}

impl Buffer {
    pub fn new_from_vec<T: Sized>(
        vulkan_context: &VulkanContext,
        data: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let item_count = data.len();
        let (handle, device_memory) =
            vulkan_context.create_buffer_with_data(data, usage, item_count)?;

        Ok(Self {
            handle,
            device_memory,
            item_count,
        })
    }

    pub fn new<T: Sized>(
        vulkan_context: &VulkanContext,
        data: &T,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let item_count = 1;
        let (handle, device_memory) =
            vulkan_context.create_buffer_with_data(data, usage, item_count)?;

        Ok(Self {
            handle,
            device_memory,
            item_count,
        })
    }
}

impl Buffer {
    pub fn destroy(&self, vulkan_context: &VulkanContext) -> () {
        let device = &vulkan_context.device;
        unsafe {
            device.destroy_buffer(self.handle, None);
            device.unmap_memory(self.device_memory);
            device.free_memory(self.device_memory, None);
        };
    }
}
