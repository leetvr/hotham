use ash::vk;

use crate::resources::{render_context::CullData, VulkanContext};
use anyhow::Result;

use super::{buffer::Buffer, descriptors::Descriptors, resources::DrawData, scene_data::SceneData};
static DRAW_DATA_BUFFER_SIZE: usize = 10_000; // TODO

/// A container for all the resources necessary to render a single frame.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The fence used to signal when the frame has completed rendering
    pub fence: vk::Fence,
    /// A command buffer used to record commands
    pub command_buffer: vk::CommandBuffer,
    /// Data for the primitives that will be drawn this frame, indexed by gl_DrawId
    pub draw_data_buffer: Buffer<DrawData>,
    /// The actual draw calls for this frame.
    pub draw_indirect_buffer: Buffer<vk::DrawIndexedIndirectCommand>,
    /// Shared data used in a scene
    pub scene_data_buffer: Buffer<SceneData>,
    /// Shared data used in a scene
    pub cull_data_buffer: Buffer<CullData>,
}

impl Frame {
    pub(crate) fn new(
        vulkan_context: &VulkanContext,
        index: usize,
        descriptors: &Descriptors,
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
                    .command_buffer_count(1)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_pool(command_pool),
            )
        }?;

        let command_buffer = command_buffers[0];

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
        let mut scene_data_buffer =
            unsafe { Buffer::new(vulkan_context, vk::BufferUsageFlags::UNIFORM_BUFFER, 1) };
        let cull_data_buffer =
            unsafe { Buffer::new(vulkan_context, vk::BufferUsageFlags::UNIFORM_BUFFER, 1) };

        // Update the descriptor sets for this frame.
        unsafe {
            draw_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.sets[index],
                0,
            );
            draw_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.compute_sets[index],
                0,
            );
            draw_indirect_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.compute_sets[index],
                1,
            );
            cull_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.compute_sets[index],
                2,
            );

            scene_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.sets[index],
                3,
            );

            // Add some default data to the scene buffer.
            scene_data_buffer.push(&Default::default());
        }

        Ok(Self {
            fence,
            command_buffer,
            draw_data_buffer,
            draw_indirect_buffer,
            scene_data_buffer,
            cull_data_buffer,
        })
    }
}
