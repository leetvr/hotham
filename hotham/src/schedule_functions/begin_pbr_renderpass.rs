use crate::{
    resources::xr_context::XrContext, resources::RenderContext, resources::VulkanContext,
    util::is_view_valid,
};
pub fn begin_pbr_renderpass(
    xr_context: &mut XrContext,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) {
    // Check if we should be rendering.
    if !xr_context.frame_state.should_render {
        println!(
            "[HOTHAM_BEGIN_PBR_RENDERPASS] - Session is runing but shouldRender is false - not rendering"
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
