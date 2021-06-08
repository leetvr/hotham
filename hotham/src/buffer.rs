use anyhow::Result;
use ash::{version::DeviceV1_0, vk};

use crate::vulkan_context::VulkanContext;

pub(crate) struct Buffer<T> {
    pub data: Vec<T>,
    pub handle: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
}

impl<T> Buffer<T>
where
    T: Sized + Clone,
{
    pub fn new(
        vulkan_context: &VulkanContext,
        data: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let (handle, device_memory) = vulkan_context.create_buffer_with_data(data, usage)?;
        let data = data.clone(); // FIXME: Cloning

        Ok(Self {
            data,
            handle,
            device_memory,
        })
    }
}

impl<T> Buffer<T> {
    pub fn destroy(&self, vulkan_context: &VulkanContext) -> () {
        let device = &vulkan_context.device;
        unsafe {
            device.destroy_buffer(self.handle, None);
            device.unmap_memory(self.device_memory);
            device.free_memory(self.device_memory, None);
        };
    }
}
