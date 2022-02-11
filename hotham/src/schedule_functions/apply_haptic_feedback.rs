use openxr::{Duration, HapticVibration};

use crate::resources::{HapticContext, XrContext};
static HAPTIC_FREQUENCY: f32 = 400.;
static HAPTIC_DURATION: i64 = 1e+8 as _; // 100ms

/// Triggers the application of vibrations to the appropriate user input device at prescribed amplitude, frequency, and duration given a Hothman::resources::XrContent and Hothman::resources::HapticContext.
///
/// During each tick of the Hotham engine, haptic feedback is applied to generate a HapticVibration
/// event which proprogates to the appropriate user input device.
///
/// Basic usage:
/// ```ignore
/// fn tick (...) {
///    apply_haptic_feedback(xr_context, haptic_context)
/// }
/// ```
pub fn apply_haptic_feedback(xr_context: &mut XrContext, haptic_context: &mut HapticContext) {
    let haptic_duration = Duration::from_nanos(HAPTIC_DURATION);
    if haptic_context.left_hand_amplitude_this_frame != 0. {
        let event = HapticVibration::new()
            .amplitude(haptic_context.left_hand_amplitude_this_frame)
            .frequency(HAPTIC_FREQUENCY)
            .duration(haptic_duration);

        xr_context
            .haptic_feedback_action
            .apply_feedback(
                &xr_context.session,
                xr_context.left_hand_subaction_path,
                &event,
            )
            .expect("Unable to apply haptic feedback!");

        // Reset the value
        haptic_context.left_hand_amplitude_this_frame = 0.;
    }

    if haptic_context.right_hand_amplitude_this_frame != 0. {
        let event = HapticVibration::new()
            .amplitude(haptic_context.right_hand_amplitude_this_frame)
            .frequency(HAPTIC_FREQUENCY)
            .duration(haptic_duration);

        xr_context
            .haptic_feedback_action
            .apply_feedback(
                &xr_context.session,
                xr_context.right_hand_subaction_path,
                &event,
            )
            .expect("Unable to apply haptic feedback!");

        // Reset the value
        haptic_context.right_hand_amplitude_this_frame = 0.;
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;

    /// Simple smoke test.
    #[test]
    pub fn apply_haptic_feedback_test() {
        let (mut xr_context, _) = XrContext::new().unwrap();
        let mut haptic_context = HapticContext::default();

        apply_haptic_feedback(&mut xr_context, &mut haptic_context);
    }
}
