use crate::{
    resources::RenderContext,
    resources::{VulkanContext, XrContext},
};
pub fn end_pbr_renderpass(
    xr_context: &mut XrContext,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) {
    // Check if we should be rendering.
    if !xr_context.frame_state.should_render {
        println!(
            "[HOTHAM_END_PBR_RENDERPASS] - Session is runing but shouldRender is false - not rendering"
        );
        return;
    }

    render_context.end_pbr_render_pass(&vulkan_context, xr_context.frame_index);
}
