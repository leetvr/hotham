use openxr::ActiveActionSet;

use crate::{
    resources::{xr_context::XrContext, RenderContext},
    util::is_view_valid,
    VIEW_TYPE,
};

/// Begin a frame
/// Make sure to call this BEFORE beginning any renderpasses.
pub fn begin_frame(
    xr_context: &mut XrContext,
    render_context: &mut RenderContext,
) -> (bool, usize) {
    let active_action_set = ActiveActionSet::new(&xr_context.input.action_set);
    xr_context
        .session
        .sync_actions(&[active_action_set])
        .unwrap();

    // Wait for a frame to become available from the runtime, then get its index.
    let swapchain_index = xr_context.begin_frame().unwrap();

    let (view_state_flags, views) = xr_context
        .session
        .locate_views(
            VIEW_TYPE,
            xr_context.frame_state.predicted_display_time,
            &xr_context.stage_space,
        )
        .unwrap();
    xr_context.views = views;
    xr_context.view_state_flags = view_state_flags;

    // If we have a valid view from OpenXR, update the scene buffers with the view data.
    if is_view_valid(&xr_context.view_state_flags) {
        let views = &xr_context.views;

        // Update uniform buffers
        render_context
            .update_scene_data(views, swapchain_index)
            .unwrap();
    }

    (xr_context.frame_state.should_render, swapchain_index)
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use crate::resources::{RenderContext, XrContext};

    use super::begin_frame;

    #[test]

    pub fn test_begin_frame() {
        let (mut xr_context, vulkan_context) = XrContext::testing();
        let mut render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();

        let (should_render, frame_index) = begin_frame(&mut xr_context, &mut render_context);
        assert!(should_render);
        assert_eq!(frame_index, 0);
    }
}
