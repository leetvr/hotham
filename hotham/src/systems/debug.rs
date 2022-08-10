use crate::resources::{InputContext, RenderContext};

/// A simple system used to assist with debugging the fragment shader.
pub fn debug_system(input_context: &InputContext, render_context: &mut RenderContext) {
    let params = &mut render_context.scene_data.params;
    if (input_context.x_button_just_pressed() || input_context.y_button_just_pressed())
        && input_context.x_button()
        && input_context.y_button()
    {
        params.z = 0.;
        params.w = 0.;
        println!("[HOTHAM_DEBUG] params.z and params.w are now 0");
    } else {
        if input_context.x_button_just_pressed() {
            params.w = 0.;
            params.z = ((params.z + 1.) % 7.) as f32;
            println!("[HOTHAM_DEBUG] params.z is now {}", params.z);
        }
        if input_context.y_button_just_pressed() {
            params.z = 0.;
            params.w = ((params.w + 1.) % 6.) as f32;
            println!("[HOTHAM_DEBUG] params.w is now {}", params.w);
        }
    }

    if (input_context.a_button_just_pressed() || input_context.b_button_just_pressed())
        && input_context.a_button()
        && input_context.b_button()
    {
        params.x = 1.;
        println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
    } else {
        if input_context.b_button_just_pressed() {
            params.x = (params.x * 10. + 1.).round() % 50. * 0.1 as f32;
            println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
        }
        if input_context.a_button_just_pressed() {
            params.x = 0.;
            println!("[HOTHAM_DEBUG] params.x is now {}", params.x);
        }
    }
}
