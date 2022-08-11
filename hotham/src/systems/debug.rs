use crate::resources::{InputContext, RenderContext};

/// A simple system used to assist with debugging the fragment shader.
pub fn debug_system(input_context: &InputContext, render_context: &mut RenderContext) {
    if input_context.x_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.w = 0.;
        params.z = ((params.z + 1.) % 7.) as f32;
        println!("[HOTHAM_DEBUG] params.z is now {}", params.z);
    }

    if input_context.y_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.z = 0.;
        params.w = ((params.w + 1.) % 6.) as f32;
        println!("[HOTHAM_DEBUG] params.w is now {}", params.w);
    }

    if input_context.b_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.x = ((params.x + 0.1) % 5.) as f32;
        println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
    }
}
