use ash::vk;

use crate::resources::VulkanContext;
use anyhow::Result;

use super::{buffer::Buffer, resources::DrawData, scene_data::SceneData};
static DRAW_DATA_BUFFER_SIZE: usize = 10_000; // TODO

/// A container for all the resources necessary to render a single frame.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The fence used to signal when the frame has completed rendering
    pub fence: vk::Fence,
    /// A command buffer used to record commands
    pub command_buffer: vk::CommandBuffer,
    /// A framebuffer to write the final image to
    pub framebuffer: vk::Framebuffer,
    /// The image view we've been handed from the swapchain
    pub swapchain_image_view: vk::ImageView,
    /// Data for the primitives that will be drawn this frame, indexed by gl_DrawId
    pub draw_data_buffer: Buffer<DrawData>,
    /// The actual draw calls for this frame.
    pub draw_indirect_buffer: Buffer<vk::DrawIndexedIndirectCommand>,
    /// Shared data used in a scene
    pub scene_data_buffer: Buffer<SceneData>,
}

impl Frame {
    pub(crate) fn new(
        vulkan_context: &VulkanContext,
        render_pass: vk::RenderPass,
        swapchain_resolution: vk::Extent2D,
        swapchain_image_view: vk::ImageView,
        depth_image_view: vk::ImageView,
        color_image_view: vk::ImageView,
    ) -> Result<Self> {
        let device = &vulkan_context.device;
        let command_pool = vulkan_context.command_pool;

        let fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED),
                None,
            )
        }?;

        let command_buffers = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_buffer_count(2)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_pool(command_pool),
            )
        }?;

        let command_buffer = command_buffers[0];
        let attachments = [color_image_view, depth_image_view, swapchain_image_view];

        let frame_buffer_create_info = vk::FramebufferCreateInfo::builder()
            .render_pass(render_pass)
            .attachments(&attachments)
            .width(swapchain_resolution.width)
            .height(swapchain_resolution.height)
            .layers(1);

        let frame_buffer = unsafe { device.create_framebuffer(&frame_buffer_create_info, None) }?;

        let draw_data_buffer = unsafe {
            Buffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                DRAW_DATA_BUFFER_SIZE,
            )
        };
        let draw_indirect_buffer = unsafe {
            Buffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER | vk::BufferUsageFlags::INDIRECT_BUFFER,
                DRAW_DATA_BUFFER_SIZE,
            )
        };
        let scene_data_buffer =
            unsafe { Buffer::new(vulkan_context, vk::BufferUsageFlags::UNIFORM_BUFFER, 1) };

        Ok(Self {
            fence,
            command_buffer,
            framebuffer: frame_buffer,
            swapchain_image_view,
            draw_data_buffer,
            draw_indirect_buffer,
            scene_data_buffer,
        })
    }
}
