use legion::system;

use crate::{
    components::{hand::Handedness, AnimationController, Hand, Transform},
    resources::XrContext,
};

#[system(for_each)]
pub fn hands(
    transform: &mut Transform,
    hand: &mut Hand,
    animation_controller: &mut AnimationController,
    #[resource] xr_context: &XrContext,
) {
    // Get our the space and path of the hand.
    let time = xr_context.frame_state.predicted_display_time;
    let (space, path) = match hand.handedness {
        Handedness::Left => (
            &xr_context.left_hand_space,
            xr_context.left_hand_subaction_path,
        ),
        Handedness::Right => (
            &xr_context.right_hand_space,
            xr_context.right_hand_subaction_path,
        ),
    };

    // Locate the hand in the space.
    let pose = space
        .locate(&xr_context.reference_space, time)
        .unwrap()
        .pose;

    // apply transform
    transform.translation = mint::Vector3::from(pose.position).into();
    transform.rotation = mint::Quaternion::from(pose.orientation).into();

    // get grip value
    let grip_value = openxr::ActionInput::get(&xr_context.grab_action, &xr_context.session, path)
        .unwrap()
        .current_state;

    // Apply to Hand
    hand.grip_value = grip_value;

    // Apply to AnimationController
    animation_controller.blend_amount = grip_value;
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, vec3, Quaternion};

    use super::*;
    use crate::resources::XrContext;

    #[test]
    pub fn test_hands_system() {
        let (xr_context, _) = XrContext::new().unwrap();
        let mut transform = Transform::default();
        let mut hand = Hand::left();
        let mut animation_controller = AnimationController::default();
        animation_controller.blend_amount = 100.0; // bogus value
        hand.grip_value = 100.0; // bogus value

        hands(
            &mut transform,
            &mut hand,
            &mut &mut animation_controller,
            &xr_context,
        );
        assert_relative_eq!(transform.translation, vec3(-0.2, 1.4, -0.5));
        assert_relative_eq!(transform.rotation, Quaternion::new(0.0, 0.0, 0.0, 0.0));
        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(animation_controller.blend_amount, 0.0);
    }
}
