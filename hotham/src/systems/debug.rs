use crate::Engine;

/// A simple system used to assist with debugging the fragment shader.
pub fn debug_system(engine: &mut Engine) {
    let input_context = &mut engine.input_context;
    let render_context = &mut engine.render_context;

    if input_context.left.x_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.w = 0.;
        params.z = ((params.z + 1.) % 7.) as f32;
        println!("[HOTHAM_DEBUG] params.z is now {}", params.z);
    }

    if input_context.left.y_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.z = 0.;
        params.w = ((params.w + 1.) % 6.) as f32;
        println!("[HOTHAM_DEBUG] params.w is now {}", params.w);
    }

    if input_context.right.b_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.x = ((params.x + 0.1) % 5.) as f32;
        println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
    }

    if input_context.right.a_button_just_pressed() {
        let params = &mut render_context.scene_data.params;
        params.x = ((params.x + 5. - 0.1) % 5.) as f32;
        println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
    }
}
