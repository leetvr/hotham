use ash::{version::DeviceV1_0, vk};

use crate::{hotham_error::HothamError, resources::VulkanContext};
use anyhow::Result;

/// A container for all the resources necessary to render a single frame.
#[derive(Debug, Clone)]
pub struct Frame {
    pub fence: vk::Fence,
    pub command_buffer: vk::CommandBuffer,
    pub framebuffer: vk::Framebuffer,
    pub swapchain_image_view: vk::ImageView,
}

impl Frame {
    pub(crate) fn new(
        vulkan_context: &VulkanContext,
        render_pass: vk::RenderPass,
        swapchain_resolution: vk::Extent2D,
        swapchain_image_view: vk::ImageView,
        depth_image_view: vk::ImageView,
    ) -> Result<Self> {
        let device = &vulkan_context.device;
        let command_pool = vulkan_context.command_pool;

        let fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
        }?;

        let command_buffer = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_pool(command_pool),
            )
        }?
        .pop()
        .ok_or(HothamError::EmptyListError)?;

        let attachments = [swapchain_image_view, depth_image_view];

        let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(swapchain_resolution.width)
            .height(swapchain_resolution.height)
            .layers(1);

        let frame_buffer = unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }?;

        Ok(Self {
            fence,
            command_buffer,
            framebuffer: frame_buffer,
            swapchain_image_view,
        })
    }

    pub(crate) fn destroy(&self, vulkan_context: &VulkanContext) {
        unsafe {
            vulkan_context
                .device
                .destroy_framebuffer(self.framebuffer, None);
            vulkan_context.device.destroy_fence(self.fence, None);
            vulkan_context
                .device
                .free_command_buffers(vulkan_context.command_pool, &[self.command_buffer]);
            vulkan_context
                .device
                .destroy_image_view(self.swapchain_image_view, None)
        }
    }
}
