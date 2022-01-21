use crate::{
    resources::RenderContext,
    resources::{VulkanContext, XrContext},
};
use legion::{Resources, World};
pub fn end_pbr_renderpass(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let xr_context = resources.get_mut::<XrContext>().unwrap();

    // Check if we should be rendering.
    if !xr_context.frame_state.should_render {
        println!(
            "[HOTHAM_END_PBR_RENDERPASS] - Session is runing but shouldRender is false - not rendering"
        );
        return;
    }

    let current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();
    render_context.end_pbr_render_pass(&vulkan_context, *current_swapchain_image_index);
}
