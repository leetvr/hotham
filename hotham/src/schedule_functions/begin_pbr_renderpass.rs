use crate::{
    resources::xr_context::XrContext, resources::RenderContext, resources::VulkanContext,
    util::is_view_valid,
};
/// Begin the PBR renderpass
/// Evaluates the provided XrContext to ensure a valid frame state and view before initiating the
/// Vulkan render pass.
/// Make sure to only call this ONCE per frame and AFTER calling `begin_frame`, BEFORE calling `end_pbr_render_pass` and BEFORE calling `end_frame`

pub fn begin_pbr_renderpass(
    xr_context: &mut XrContext,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) {
    // Check if we should be rendering.
    if !xr_context.frame_state.should_render {
        println!(
            "[HOTHAM_BEGIN_PBR_RENDERPASS] - Session is running but shouldRender is false - not rendering"
        );
        return;
    }

    // If we have a valid view from OpenXR, update the scene buffers with the view data.
    if is_view_valid(&xr_context.view_state_flags) {
        let views = &xr_context.views;

        // Update uniform buffers
        render_context
            .update_scene_data(&views, &vulkan_context)
            .unwrap();
    }

    // TODO: This begs the question: what if we never get a valid view from OpenXR..?

    // Begin the renderpass.
    render_context.begin_pbr_render_pass(&vulkan_context, xr_context.frame_index);
    // ..and we're off!
}
