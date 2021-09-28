use crate::{resources::RenderContext, resources::VulkanContext, resources::XrContext};
use legion::{Resources, World};

pub fn end_frame(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut xr_context = resources.get_mut::<XrContext>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let swapchain_image_index = resources.get::<usize>().unwrap();
    render_context.end_render_pass(&vulkan_context, *swapchain_image_index);
    xr_context.end_frame().unwrap();
}

#[cfg(test)]
mod tests {
    use legion::{Resources, World};

    use crate::{
        resources::{RenderContext, XrContext},
        schedule_functions::begin_frame,
    };

    use super::end_frame;

    #[test]
    pub fn test_end_frame() {
        let (xr_context, vulkan_context) = XrContext::new().unwrap();
        let renderer = RenderContext::new(&vulkan_context, &xr_context).unwrap();

        let mut world = World::default();
        let mut resources = Resources::default();
        let dummy_frame_value = 0 as usize;

        resources.insert(xr_context);
        resources.insert(vulkan_context);
        resources.insert(renderer);
        resources.insert(dummy_frame_value);

        begin_frame(&mut world, &mut resources);
        end_frame(&mut world, &mut resources);
    }
}
