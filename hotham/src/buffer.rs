use std::marker::PhantomData;

use anyhow::Result;
use ash::{version::DeviceV1_0, vk};

use crate::vulkan_context::VulkanContext;

// TODO: Let Buffer<T> own the data
#[derive(Debug, Clone)]
pub(crate) struct Buffer<T> {
    pub handle: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub _phantom: PhantomData<T>,
}

impl<T> Buffer<T>
where
    T: Sized,
{
    pub fn new_from_vec(
        vulkan_context: &VulkanContext,
        data: &Vec<T>,
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let item_count = data.len();
        let (handle, device_memory) =
            vulkan_context.create_buffer_with_data(data.as_ptr(), usage, item_count)?;

        Ok(Self {
            handle,
            device_memory,
            _phantom: PhantomData,
        })
    }

    pub fn new(
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
            _phantom: PhantomData,
        })
    }

    /// **NOTE**: If passing in a Vec, you MUST use vec.as_ptr(), passing in
    /// a reference will result in A Very Bad Time.
    pub fn update(
        &self,
        vulkan_context: &VulkanContext,
        data: *const T,
        item_count: usize,
    ) -> Result<()> {
        vulkan_context.update_buffer(data, item_count, self.device_memory)
    }
}

impl<T> Buffer<T> {
    pub fn destroy(&self, vulkan_context: &VulkanContext) -> () {
        let device = &vulkan_context.device;
        unsafe {
            device.destroy_buffer(self.handle, None);
            device.free_memory(self.device_memory, None);
        };
    }
}
