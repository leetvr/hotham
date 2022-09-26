use crate::components::hand::Handedness;

/// Wrapper around XR Haptics
#[derive(Clone, Debug, Default)]
pub struct HapticContext {
    /// Haptics that should be applied to the left hand
    pub left_hand_amplitude_this_frame: f32,
    /// Haptics that should be applied to the right hand
    pub right_hand_amplitude_this_frame: f32,
}

impl HapticContext {
    /// Request haptics be applied this frame
    pub fn request_haptic_feedback(&mut self, amplitude: f32, handedness: Handedness) {
        match handedness {
            Handedness::Left => {
                if amplitude > self.left_hand_amplitude_this_frame {
                    self.left_hand_amplitude_this_frame = amplitude;
                }
            }
            Handedness::Right => {
                if amplitude > self.right_hand_amplitude_this_frame {
                    self.right_hand_amplitude_this_frame = amplitude;
                }
            }
        }
    }
}
