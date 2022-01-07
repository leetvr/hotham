use crate::{resources::RenderContext, resources::VulkanContext};
use legion::{Resources, World};
pub fn end_pbr_renderpass(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();
    render_context.end_pbr_render_pass(&vulkan_context, *current_swapchain_image_index);
}
