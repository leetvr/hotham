use core::ptr::copy_nonoverlapping;

use ash::vk;
use vulkan_context::VulkanContext;

use crate::contexts::vulkan_context;

use super::memory::allocate_memory;

/// A wrapper around a chunk of allocated memory on the GPU
#[derive(Debug, Clone)]
pub struct Buffer<T: Sized> {
    /// A handle to the underlying vk::Buffer
    pub buffer: vk::Buffer,
    /// A handle to the underlying memory
    pub device_memory: vk::DeviceMemory,
    /// A pointer to the start of the memory
    pub memory_address: std::ptr::NonNull<T>,
    /// The current length of the buffer
    pub len: usize,
    /// The maximum length of the buffer
    pub max_len: usize,
    /// Flags describing the buffer's usage
    pub usage: vk::BufferUsageFlags,
}

impl<T: Sized> Buffer<T> {
    /// Create a new buffer
    pub unsafe fn new(
        vulkan_context: &VulkanContext,
        usage: vk::BufferUsageFlags,
        max_len: usize,
    ) -> Buffer<T> {
        let device = &vulkan_context.device;

        let size = (std::mem::size_of::<T>() * max_len) as _;
        println!("[HOTHAM_VULKAN] Creating {size} buffer with flags {usage:?}..");

        let buffer = device
            .create_buffer(
                &vk::BufferCreateInfo::default().usage(usage).size(size),
                None,
            )
            .unwrap();

        let memory_requirements = device.get_buffer_memory_requirements(buffer);
        let flags = vk::MemoryPropertyFlags::HOST_VISIBLE;
        let device_memory = allocate_memory(vulkan_context, memory_requirements, flags);

        // Bind memory
        device.bind_buffer_memory(buffer, device_memory, 0).unwrap();

        // Map memory
        let memory_address = device
            .map_memory(
                device_memory,
                0,
                vk::WHOLE_SIZE,
                vk::MemoryMapFlags::empty(),
            )
            .unwrap();

        // Stash the pointer so that we can easily access it again.
        let memory_address = std::mem::transmute::<*mut std::ffi::c_void, *mut T>(memory_address);
        let memory_address = std::ptr::NonNull::new_unchecked(memory_address);

        Self {
            buffer,
            device_memory,
            memory_address,
            len: 0,
            max_len,
            usage,
        }
    }

    /// Dumb update - overrides the content of the GPU buffer with `data`.
    /// SAFETY: Unchecked! The caller MUST ensure `data` is valid and that `data.len` does not exceed `self.max_len`
    pub unsafe fn overwrite(&mut self, data: &[T]) {
        copy_nonoverlapping(data.as_ptr(), self.memory_address.as_ptr(), data.len());
        self.len = data.len();
    }

    /// Dumb append - appends the content of the GPU buffer with `data`.
    /// SAFETY: Unchecked! The caller MUST ensure `data` is valid and that `self.len + data.len` does not exceed `self.max_len`
    pub unsafe fn append(&mut self, data: &[T]) {
        copy_nonoverlapping(
            data.as_ptr(),
            self.memory_address.as_ptr().add(self.len),
            data.len(),
        );
        self.len += data.len();
    }

    /// Dumb push - adds `data` to the GPU buffer.
    /// SAFETY: Unchecked! The caller MUST ensure `data` is valid and that `self.len + 1` does not exceed `self.max_len`
    pub unsafe fn push(&mut self, data: &T) -> u32 {
        let index = self.len as u32;
        copy_nonoverlapping(
            data as _,
            self.memory_address.as_ptr().offset(index as _),
            1,
        );
        self.len += 1;
        index
    }

    /// Get the buffer's underlying data as a slice
    pub unsafe fn as_slice(&self) -> &[T] {
        std::slice::from_raw_parts(self.memory_address.as_ptr(), self.len)
    }

    /// Get the buffer's underlying data as a mutable slice
    pub unsafe fn as_slice_mut(&mut self) -> &mut [T] {
        std::slice::from_raw_parts_mut(self.memory_address.as_ptr(), self.len)
    }

    /// safety: After calling this function the buffer will be in an UNUSABLE state
    pub unsafe fn destroy(&mut self, device: &ash::Device) {
        device.unmap_memory(self.device_memory);
        device.free_memory(self.device_memory, None);
        device.destroy_buffer(self.buffer, None);
        self.len = 0;
    }

    /// Clear the buffer.
    pub fn clear(&mut self) {
        self.len = 0;
    }

    /// The length of the buffer
    pub fn len(&self) -> usize {
        self.len
    }

    /// Is the buffer empty?
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Write to the specified descriptor set
    pub unsafe fn update_descriptor_set(
        &self,
        device: &ash::Device,
        descriptor_set: vk::DescriptorSet,
        binding: u32,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::default()
            .buffer(self.buffer)
            .offset(0)
            .range(vk::WHOLE_SIZE);

        let descriptor_type = if self.usage.contains(vk::BufferUsageFlags::UNIFORM_BUFFER) {
            vk::DescriptorType::UNIFORM_BUFFER
        } else {
            vk::DescriptorType::STORAGE_BUFFER
        };

        let write = vk::WriteDescriptorSet::default()
            .buffer_info(std::slice::from_ref(&buffer_info))
            .dst_set(descriptor_set)
            .dst_binding(binding)
            .descriptor_type(descriptor_type);

        device.update_descriptor_sets(std::slice::from_ref(&write), &[]);
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::*;

    #[test]
    pub fn buffer_smoke_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        unsafe {
            let mut buffer: Buffer<usize> = Buffer::new(
                &vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                10_000,
            );

            // First, write some data into the buffer.
            let initial_data = vec![1, 2, 3, 4];
            buffer.overwrite(&initial_data);
            assert_eq!(buffer.as_slice(), &initial_data);

            // Next, overwrite it
            let next_data = vec![4, 5, 6, 7];
            buffer.overwrite(&next_data);
            assert_eq!(buffer.as_slice(), &next_data);

            // Then, append to it
            let mut additional_data = vec![8, 9, 10, 11];
            buffer.append(&additional_data);
            let mut expected_data = next_data.clone();
            expected_data.append(&mut additional_data);
            assert_eq!(buffer.as_slice(), &expected_data);

            // Then, push a bunch of shit to it
            let push_data = vec![12, 13, 14, 15, 16, 17, 18, 19, 20];
            for n in &push_data {
                buffer.push(n);
                expected_data.push(*n);
                assert_eq!(buffer.as_slice(), &expected_data);
            }

            // Clear it
            buffer.clear();
            let mut expected_data = vec![];
            assert_eq!(buffer.as_slice().len(), 0);

            // Push data again.
            let push_data = vec![12, 13, 14, 15, 16, 17, 18, 19, 20];
            for n in &push_data {
                buffer.push(n);
                expected_data.push(*n);
                assert_eq!(buffer.as_slice(), &expected_data);
            }

            // Modify element-wise
            let expected_data = vec![2, 3, 4, 5, 6, 7, 8, 9, 0];
            for (index, n) in buffer.as_slice_mut().iter_mut().enumerate() {
                *n = expected_data[index];
            }
            assert_eq!(buffer.as_slice(), &expected_data);

            // Modify by index
            let expected_data = vec![22, 23, 24, 25, 26, 27, 28, 29, 20];
            let data = buffer.as_slice_mut();
            for (index, n) in expected_data.iter().enumerate() {
                data[index] = *n;
            }
            assert_eq!(buffer.as_slice(), &expected_data);
        }
    }
}
