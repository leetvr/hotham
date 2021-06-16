use std::{marker::PhantomData, mem::size_of};

use anyhow::Result;
use ash::{version::DeviceV1_0, vk};

use crate::{hotham_error::HothamError, vulkan_context::VulkanContext};

// TODO: Let Buffer<T> own the data
pub(crate) struct Buffer<T> {
    pub handle: vk::Buffer,
    pub device_memory: vk::DeviceMemory,
    pub item_count: usize,
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
            item_count,
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
            item_count,
            _phantom: PhantomData,
        })
    }

    pub fn update(
        &mut self,
        vulkan_context: &VulkanContext,
        data: &T,
        item_count: usize,
    ) -> Result<()> {
        self.item_count = item_count;
        vulkan_context.update_buffer(data, item_count, self.device_memory)
    }

    pub fn create_descriptor_set(
        &self,
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
    ) -> Result<vk::DescriptorSet> {
        let descriptor_set = unsafe {
            vulkan_context.device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .set_layouts(set_layouts)
                    .descriptor_pool(vulkan_context.descriptor_pool),
            )
        }?
        .pop()
        .ok_or(HothamError::EmptyListError)?;

        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(self.handle)
            .offset(0)
            .range(size_of::<T>() as _)
            .build();

        let descriptor_write = vk::WriteDescriptorSet::builder()
            .dst_set(descriptor_set)
            .dst_binding(0)
            .dst_array_element(0)
            .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
            .buffer_info(&[buffer_info])
            .build();

        unsafe {
            vulkan_context
                .device
                .update_descriptor_sets(&[descriptor_write], &[])
        };

        Ok(descriptor_set)
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
