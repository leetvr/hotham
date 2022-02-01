use crate::components::hand::Handedness;

#[derive(Clone, Debug, Default)]
pub struct HapticContext {
    pub left_hand_amplitude_this_frame: f32,
    pub right_hand_amplitude_this_frame: f32,
}

impl HapticContext {
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
