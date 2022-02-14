use crate::{
    resources::RenderContext,
    resources::{VulkanContext, XrContext},
};
/// End the PBR renderpass
/// Evalues the provided XrContent to ensure the frame state allows for rendering before completing
/// the Vulkan render pass.
/// Make sure to only call this function ONCE per frame, AFTER `begin_frame`, AFTER `begin_pbr_renderpass` and BEFORE `end_frame`

pub fn end_pbr_renderpass(
    xr_context: &mut XrContext,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) {
    // Check if we should be rendering.
    if !xr_context.frame_state.should_render {
        println!(
            "[HOTHAM_END_PBR_RENDERPASS] - Session is running but shouldRender is false - not rendering"
        );
        return;
    }

    render_context.end_pbr_render_pass(vulkan_context, xr_context.frame_index);
}
