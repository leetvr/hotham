use legion::{Resources, World};
use openxr::{Duration, HapticVibration};

use crate::resources::{HapticContext, XrContext};

pub fn apply_haptic_feedback(_: &mut World, resources: &mut Resources) {
    let mut haptic_context = resources
        .get_mut::<HapticContext>()
        .expect("Unable to get HapticContext");

    if haptic_context.amplitude_this_frame == 0. {
        return;
    }

    let xr_context = resources
        .get_mut::<XrContext>()
        .expect("Unable to get XrContext");

    let duration = Duration::from_nanos(1e+7 as _);
    let frequency = 180.;

    let event = HapticVibration::new()
        .amplitude(haptic_context.amplitude_this_frame)
        .frequency(frequency)
        .duration(duration);

    xr_context
        .haptic_feedback_action
        .apply_feedback(
            &xr_context.session,
            xr_context.right_hand_subaction_path,
            &event,
        )
        .expect("Unable to apply haptic feedback!");

    // Reset the value
    haptic_context.amplitude_this_frame = 0.;
}
