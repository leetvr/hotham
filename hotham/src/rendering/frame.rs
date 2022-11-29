use ash::vk;

use crate::contexts::{render_context::CullParams, VulkanContext};
use anyhow::Result;

use super::{
    buffer::{Buffer, DeviceLocalBuffer},
    descriptors::{
        Descriptors, CULL_PARAMS_BINDING, DRAW_DATA_BINDING, PRIMITIVE_CULL_DATA_BINDING,
        SCENE_DATA_BINDING,
    },
    resources::{DrawData, PrimitiveCullData},
    scene_data::SceneData,
};

// We *can* draw this many objects, but.. seriously?
static DRAW_DATA_BUFFER_SIZE: usize = 5000;

// We *can* draw this many objects, but.. seriously?
static PRIMITIVE_CULL_DATA_BUFFER_SIZE: usize = 100_000;

/// A container for all the resources necessary to render a single frame.
#[derive(Debug, Clone)]
pub struct Frame {
    /// The fence used to signal when the frame has completed rendering
    pub fence: vk::Fence,
    /// A command buffer used to record commands
    pub command_buffer: vk::CommandBuffer,
    /// The fence used to signal when the frame has completed rendering
    pub compute_fence: vk::Fence,
    /// A command buffer used to record commands
    pub compute_command_buffer: vk::CommandBuffer,
    /// Data for the primitives that will be drawn this frame, indexed by gl_InstanceId
    pub draw_data_buffer: DeviceLocalBuffer<DrawData>,
    /// The actual draw calls for this frame.
    pub primitive_cull_data_buffer: DeviceLocalBuffer<PrimitiveCullData>,
    /// Shared data used in a scene
    pub scene_data_buffer: DeviceLocalBuffer<SceneData>,
    /// Shared data used in a scene
    pub cull_params_buffer: DeviceLocalBuffer<CullParams>,
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
        let compute_fence = unsafe { device.create_fence(&vk::FenceCreateInfo::builder(), None) }?;

        let command_buffers = unsafe {
            device.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::builder()
                    .command_buffer_count(2)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_pool(command_pool),
            )
        }?;

        let command_buffer = command_buffers[0];
        let compute_command_buffer = command_buffers[1];

        let draw_data_buffer = unsafe {
            DeviceLocalBuffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                DRAW_DATA_BUFFER_SIZE,
            )
        };
        let primitive_cull_data_buffer = unsafe {
            DeviceLocalBuffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                PRIMITIVE_CULL_DATA_BUFFER_SIZE,
            )
        };
        let mut scene_data_buffer = unsafe {
            DeviceLocalBuffer::new(vulkan_context, vk::BufferUsageFlags::UNIFORM_BUFFER, 1)
        };
        let cull_params_buffer = unsafe {
            DeviceLocalBuffer::new(vulkan_context, vk::BufferUsageFlags::UNIFORM_BUFFER, 1)
        };

        // Update the descriptor sets for this frame.
        unsafe {
            // Graphics
            draw_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.sets[index],
                DRAW_DATA_BINDING,
            );
            scene_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.sets[index],
                SCENE_DATA_BINDING,
            );

            // Compute
            primitive_cull_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.compute_sets[index],
                PRIMITIVE_CULL_DATA_BINDING,
            );
            cull_params_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.compute_sets[index],
                CULL_PARAMS_BINDING,
            );

            // Add some default data to the scene buffer.
            scene_data_buffer.push(&Default::default());
        }

        Ok(Self {
            fence,
            compute_fence,
            command_buffer,
            compute_command_buffer,
            draw_data_buffer,
            primitive_cull_data_buffer,
            scene_data_buffer,
            cull_params_buffer,
        })
    }
}
