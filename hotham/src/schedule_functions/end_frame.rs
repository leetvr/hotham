use crate::{resources::RenderContext, resources::VulkanContext, resources::XrContext};
use legion::{Resources, World};

pub fn end_frame(_world: &mut World, resources: &mut Resources) {
    // Get resources
    let mut xr_context = resources.get_mut::<XrContext>().unwrap();
    let mut render_context = resources.get_mut::<RenderContext>().unwrap();
    let current_swapchain_image_index = resources.get_mut::<usize>().unwrap();
    let vulkan_context = resources.get::<VulkanContext>().unwrap();

    // Check if we should be rendering.
    if xr_context.frame_state.should_render {
        render_context.end_frame(&vulkan_context, *current_swapchain_image_index);
    } else {
        println!(
            "[HOTHAM_END_FRAME] - Session is runing but shouldRender is false - not rendering"
        );
    }
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
