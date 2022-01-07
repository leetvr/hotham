use crate::{resources::xr_context::XrContext, resources::RenderContext, resources::VulkanContext};
use legion::{Resources, World};
pub fn begin_pbr_renderpass(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let xr_context = resources.get_mut::<XrContext>().unwrap();
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();

    // Get views from OpenXR
    let views = &xr_context.views;

    // Update uniform buffers
    render_context
        .update_scene_data(&views, &vulkan_context)
        .unwrap();

    // Begin the renderpass.
    render_context.begin_pbr_render_pass(&vulkan_context, *current_swapchain_image_index);
    // ..and we're off!
}
