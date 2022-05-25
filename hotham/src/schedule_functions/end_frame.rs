use crate::{resources::RenderContext, resources::VulkanContext, resources::XrContext};

/// End the current frame
/// Make sure to ONLY call this AFTER `begin_frame` and DO NOT issue any further rendering commands this frame
pub fn end_frame(
    xr_context: &mut XrContext,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) {
    // Check if we should be rendering.
    if xr_context.frame_state.should_render {
        render_context.end_frame(vulkan_context, xr_context.frame_index);
    } else {
        println!(
            "[HOTHAM_END_FRAME] - Session is running but shouldRender is false - not rendering"
        );
    }
    xr_context.end_frame().unwrap();
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {

    use super::*;
    use crate::{
        resources::{RenderContext, XrContext},
        schedule_functions::begin_frame,
    };

    #[test]
    pub fn test_end_frame() {
        let (mut xr_context, vulkan_context) = XrContext::new().unwrap();
        let mut render_context = RenderContext::new(&vulkan_context, &xr_context).unwrap();
        begin_frame(&mut xr_context, &vulkan_context, &render_context);
        end_frame(&mut xr_context, &vulkan_context, &mut render_context);
    }
}
