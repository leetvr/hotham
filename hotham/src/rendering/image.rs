use ash::vk;

/// Thin wrapper around a locally created Vulkan image.
#[derive(Debug, Clone)]
pub struct Image {
    /// Handle to the underlying Vulkan image
    pub handle: vk::Image,
    /// Handle to an ImageView for this image
    pub view: vk::ImageView,
    /// Handle to the underlying image
    pub device_memory: vk::DeviceMemory,
    /// The extent (size) of this image
    pub extent: vk::Extent2D,
    /// Flags to indicate to the renderer how this image will be used
    pub usage: vk::ImageUsageFlags,
    /// The format of this image
    pub format: vk::Format,
    /// The type of ImageView
    pub view_type: vk::ImageViewType,
    /// The number of layers in the image
    pub layer_count: u32,
}

impl Image {
    /// Create a new image
    /// TODO: At the moment the "logic" for this function is handled by `vulkan_context`. We should change that.
    #[allow(clippy::too_many_arguments)]
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
}
