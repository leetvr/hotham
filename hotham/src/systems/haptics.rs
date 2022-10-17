use openxr::{Duration, HapticVibration};

use crate::{
    contexts::{HapticContext, XrContext},
    Engine,
};
static HAPTIC_FREQUENCY: f32 = 400.;
static HAPTIC_DURATION: i64 = 1e+8 as _; // 100ms

/// Triggers the application of vibrations to the appropriate user input device at prescribed amplitude, frequency, and duration given a Hotham::resources::XrContent and Hotham::resources::HapticContext.
///
/// During each tick of the Hotham engine, haptic feedback is applied to generate a HapticVibration
/// event which propagates to the appropriate user input device.
///
/// Basic usage:
/// ```ignore
/// fn tick (...) {
///    apply_haptic_feedback(xr_context, haptic_context)
/// }
/// ```
pub fn haptics_system(engine: &mut Engine) {
    haptics_system_inner(&mut engine.xr_context, &mut engine.haptic_context)
}

fn haptics_system_inner(xr_context: &mut XrContext, haptic_context: &mut HapticContext) {
    let input = &xr_context.input;

    let haptic_duration = Duration::from_nanos(HAPTIC_DURATION);
    if haptic_context.left_hand_amplitude_this_frame != 0. {
        let event = HapticVibration::new()
            .amplitude(haptic_context.left_hand_amplitude_this_frame)
            .frequency(HAPTIC_FREQUENCY)
            .duration(haptic_duration);

        input
            .haptic_feedback_action
            .apply_feedback(&xr_context.session, input.left_hand_subaction_path, &event)
            .expect("Unable to apply haptic feedback!");

        // Reset the value
        haptic_context.left_hand_amplitude_this_frame = 0.;
    }

    if haptic_context.right_hand_amplitude_this_frame != 0. {
        let event = HapticVibration::new()
            .amplitude(haptic_context.right_hand_amplitude_this_frame)
            .frequency(HAPTIC_FREQUENCY)
            .duration(haptic_duration);

        input
            .haptic_feedback_action
            .apply_feedback(&xr_context.session, input.right_hand_subaction_path, &event)
            .expect("Unable to apply haptic feedback!");

        // Reset the value
        haptic_context.right_hand_amplitude_this_frame = 0.;
    }
}
