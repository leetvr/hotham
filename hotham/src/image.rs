use ash::{version::DeviceV1_0, vk};

use crate::resources::VulkanContext;

/// Thin wrapper around a locally created Vulkan image.
#[derive(Debug, Clone)]
pub struct Image {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,
    pub extent: vk::Extent2D,
}

impl Image {
    pub fn new(
        handle: vk::Image,
        view: vk::ImageView,
        device_memory: vk::DeviceMemory,
        extent: vk::Extent2D,
    ) -> Self {
        Self {
            handle,
            view,
            device_memory,
            extent,
        }
    }

    pub(crate) fn destroy(&self, vulkan_context: &VulkanContext) {
        let device = &vulkan_context.device;
        unsafe {
            device.free_memory(self.device_memory, None);
            device.destroy_image_view(self.view, None);
            device.destroy_image(self.handle, None);
        };
    }
}
