use std::marker::PhantomData;

use anyhow::Result;
use ash::vk;

use crate::resources::VulkanContext;

// TODO: Let Buffer<T> own the data
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub struct Buffer<T> {
    pub handle: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub _phantom: PhantomData<T>,
    pub size: vk::DeviceSize,
    pub device_memory_size: vk::DeviceSize,
    pub usage: vk::BufferUsageFlags,
}

impl<T> Buffer<T>
where
    T: Sized + Copy,
{
    pub fn new(
        vulkan_context: &VulkanContext,
        data: &[T],
        usage: vk::BufferUsageFlags,
    ) -> Result<Self> {
        let size = std::mem::size_of_val(data) as vk::DeviceSize;
        let (handle, device_memory, device_memory_size) =
            vulkan_context.create_buffer_with_data(data, usage, size)?;

        Ok(Self {
            handle,
            device_memory,
            size,
            device_memory_size,
            usage,
            _phantom: PhantomData,
        })
    }

    /// **NOTE**: If passing in a Vec, you MUST use vec.as_ptr(), passing in
    /// a reference will result in A Very Bad Time.
    pub fn update(&self, vulkan_context: &VulkanContext, data: &[T]) -> Result<()> {
        vulkan_context.update_buffer(
            data,
            self.device_memory,
            self.device_memory_size,
            self.usage,
        )
    }
}

// TODO: Need to be able to drop Buffers
// impl<T> Buffer<T> {
//     pub(crate) fn destroy(&self, vulkan_context: &VulkanContext) -> () {
//         let device = &vulkan_context.device;
//         unsafe {
//             device.destroy_buffer(self.handle, None);
//             device.free_memory(self.device_memory, None);
//         };
//     }
// }
