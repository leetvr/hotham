use legion::{Resources, World};
use openxr::ActiveActionSet;

use crate::{
    resources::xr_context::XrContext, resources::RenderContext, resources::VulkanContext, VIEW_TYPE,
};

pub fn begin_frame(_world: &mut World, resources: &mut Resources) {
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
    // TODO: Push current_swapchain_image_index into XrContext
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

    // Begin the renderpass.
    render_context.begin_render_pass(&vulkan_context, available_swapchain_image_index);
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
