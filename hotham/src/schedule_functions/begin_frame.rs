use legion::{Resources, World};
use openxr::ActiveActionSet;

use crate::{
    resources::{xr_context::XrContext, RenderContext, VulkanContext},
    VIEW_TYPE,
};

pub fn begin_frame(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut xr_context = resources.get_mut::<XrContext>().unwrap();
    let mut current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let render_context = resources.get_mut::<RenderContext>().unwrap();
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

    let (view_state_flags, views) = xr_context
        .session
        .locate_views(
            VIEW_TYPE,
            frame_state.predicted_display_time,
            &xr_context.reference_space,
        )
        .unwrap();
    xr_context.views = views;
    xr_context.view_state_flags = view_state_flags;

    // If the shouldRender flag is set, start rendering
    if xr_context.frame_state.should_render {
        render_context.begin_frame(&vulkan_context, available_swapchain_image_index);
    } else {
        println!(
            "[HOTHAM_BEGIN_FRAME] - Session is runing but shouldRender is false - not rendering"
        );
    }
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
