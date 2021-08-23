use std::time::Instant;

use crate::{resources::RenderContext, resources::VulkanContext, resources::XrContext, BLEND_MODE};
use ash::{version::DeviceV1_0, vk};
use legion::{Resources, World};
use openxr as xr;

pub(crate) fn end_frame(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut xr_context = resources.get_mut::<XrContext>().unwrap();
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let swapchain_image_index = resources.get::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();

    // Get the values we need to end the renderpass
    let device = &vulkan_context.device;
    let frame = &render_context.frames[*swapchain_image_index];
    let command_buffer = frame.command_buffer;
    let graphics_queue = vulkan_context.graphics_queue;

    // End the Vulkan RenderPass
    // TODO: Should we split this into a Vulkan specific function?
    unsafe {
        device.cmd_end_render_pass(command_buffer);
        device.end_command_buffer(command_buffer).unwrap();
        let fence = frame.fence;
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
        device.reset_fences(&[fence]).unwrap();
        device
            .queue_submit(graphics_queue, &[submit_info], fence)
            .unwrap();
        device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
    }

    render_context.last_frame_time = Instant::now();

    // Submit the image to OpenXR
    xr_context.swapchain.release_image().unwrap();

    let rect = xr::Rect2Di {
        offset: xr::Offset2Di { x: 0, y: 0 },
        extent: xr::Extent2Di {
            width: xr_context.swapchain_resolution.width as _,
            height: xr_context.swapchain_resolution.height as _,
        },
    };

    let display_time = xr_context.frame_state.predicted_display_time;

    let views = [
        xr::CompositionLayerProjectionView::new()
            .pose(xr_context.views[0].pose)
            .fov(xr_context.views[0].fov)
            .sub_image(
                xr::SwapchainSubImage::new()
                    .swapchain(&xr_context.swapchain)
                    .image_array_index(0)
                    .image_rect(rect),
            ),
        xr::CompositionLayerProjectionView::new()
            .pose(xr_context.views[1].pose)
            .fov(xr_context.views[1].fov)
            .sub_image(
                xr::SwapchainSubImage::new()
                    .swapchain(&xr_context.swapchain)
                    .image_array_index(1)
                    .image_rect(rect),
            ),
    ];

    let layer_projection = xr::CompositionLayerProjection::new()
        .space(&xr_context.reference_space)
        .views(&views);

    // Annoying but necessary due to lifetimes on xr_context.
    let projection = unsafe { std::mem::transmute(&layer_projection.into_raw()) };

    xr_context
        .frame_stream
        .end(display_time, BLEND_MODE, &[projection])
        .unwrap();
}

#[cfg(test)]
mod tests {
    use legion::{Resources, World};

    use crate::{
        resources::{RenderContext, XrContext},
        schedule_functions::begin_frame,
    };

    use super::end_frame;

    #[test]
    pub fn test_end_frame() {
        let (xr_context, vulkan_context) = XrContext::new().unwrap();
        let renderer = RenderContext::new(&vulkan_context, &xr_context).unwrap();

        let mut world = World::default();
        let mut resources = Resources::default();
        let dummy_frame_value = 0 as usize;

        resources.insert(xr_context);
        resources.insert(vulkan_context);
        resources.insert(renderer);
        resources.insert(dummy_frame_value);

        begin_frame(&mut world, &mut resources);
        end_frame(&mut world, &mut resources);
    }
}
