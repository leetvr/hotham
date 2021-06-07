use ash::{version::DeviceV1_0, vk};

use crate::{hotham_error::HothamError, vulkan_context::VulkanContext};
use anyhow::Result;

#[derive(Debug, Clone)]
pub(crate) struct Frame {
    pub fence: vk::Fence,
    pub command_buffer: vk::CommandBuffer,
    pub frame_buffer: vk::Framebuffer,
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
            frame_buffer,
        })
    }
}
