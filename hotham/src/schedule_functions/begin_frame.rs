use ash::{version::DeviceV1_0, vk};
use legion::{Resources, World};
use openxr::ActiveActionSet;

use crate::{
    resources::xr_context::XrContext, resources::RenderContext, resources::VulkanContext, VIEW_TYPE,
};

static CLEAR_VALUES: [vk::ClearValue; 2] = [
    vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    },
    vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    },
];

pub(crate) fn begin_frame(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut xr_context = resources.get_mut::<XrContext>().unwrap();
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let mut current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();

    let active_action_set = ActiveActionSet::new(&xr_context.action_set);
    xr_context
        .session
        .sync_actions(&[active_action_set])
        .unwrap();

    // Wait for a frame to become available from the runtime
    // TODO: Push current_swapchain_image_index into RenderContext
    let (frame_state, available_swapchain_image_index) = xr_context.begin_frame().unwrap();
    (*current_swapchain_image_index) = available_swapchain_image_index;
    xr_context.frame_state = frame_state;

    let (_, views) = xr_context
        .session
        .locate_views(
            VIEW_TYPE,
            frame_state.predicted_display_time,
            &xr_context.reference_space,
        )
        .unwrap();

    // Update uniform buffers
    // TODO: We should do this ourselves.
    render_context
        .update_scene_data(&views, &vulkan_context)
        .unwrap();
    xr_context.views = views;

    // Get the values we need to start a renderpass
    let device = &vulkan_context.device;
    let frame = &render_context.frames[available_swapchain_image_index];
    let command_buffer = frame.command_buffer;
    let framebuffer = frame.framebuffer;

    // Begin the renderpass.
    let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
        .render_pass(render_context.render_pass)
        .framebuffer(framebuffer)
        .render_area(render_context.render_area)
        .clear_values(&CLEAR_VALUES);

    unsafe {
        device
            .begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )
            .unwrap();
        device.cmd_begin_render_pass(
            command_buffer,
            &render_pass_begin_info,
            vk::SubpassContents::INLINE,
        );
        device.cmd_bind_pipeline(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            render_context.pipeline,
        );
        device.cmd_bind_descriptor_sets(
            command_buffer,
            vk::PipelineBindPoint::GRAPHICS,
            render_context.pipeline_layout,
            0,
            &render_context.scene_data_descriptor_sets,
            &[],
        );
    }

    // ..and we're off!
}

#[cfg(test)]
mod tests {
    use legion::{Resources, World};

    use crate::resources::{RenderContext, XrContext};

    use super::begin_frame;

    #[test]
    pub fn test_begin_frame() {
        let (xr_context, vulkan_context) = XrContext::new().unwrap();
        let render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();

        let mut world = World::default();
        let mut resources = Resources::default();
        let dummy_frame_value = 100 as usize;

        resources.insert(xr_context);
        resources.insert(vulkan_context);
        resources.insert(render_context);
        resources.insert(dummy_frame_value);

        begin_frame(&mut world, &mut resources);
        let current_frame = resources.get::<usize>().unwrap();
        assert_ne!(*current_frame, dummy_frame_value);
    }
}
