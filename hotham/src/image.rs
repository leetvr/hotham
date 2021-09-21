use ash::vk;

/// Thin wrapper around a locally created Vulkan image.
#[derive(Debug, Clone)]
pub struct Image {
    pub handle: vk::Image,
    pub view: vk::ImageView,
    pub device_memory: vk::DeviceMemory,
    pub extent: vk::Extent2D,
    pub usage: vk::ImageUsageFlags,
    pub format: vk::Format,
    pub view_type: vk::ImageViewType,
    pub layer_count: u32,
}

impl Image {
    pub fn new(
        handle: vk::Image,
        view: vk::ImageView,
        device_memory: vk::DeviceMemory,
        extent: vk::Extent2D,
        usage: vk::ImageUsageFlags,
        format: vk::Format,
        view_type: vk::ImageViewType,
        layer_count: u32,
    ) -> Self {
        Self {
            handle,
            view,
            device_memory,
            extent,
            usage,
            format,
            view_type,
            layer_count,
        }
    }

    // TODO: Handle destruction
    // pub(crate) fn destroy(&self, vulkan_context: &VulkanContext) {
    //     let device = &vulkan_context.device;
    //     unsafe {
    //         device.free_memory(self.device_memory, None);
    //         device.destroy_image_view(self.view, None);
    //         device.destroy_image(self.handle, None);
    //     };
    // }
}
