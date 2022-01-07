#[derive(Clone, Debug, Default)]
pub struct HapticContext {
    pub amplitude_this_frame: f32,
}

impl HapticContext {
    pub fn request_haptic_feedback(&mut self, amplitude: f32) {
        if amplitude > self.amplitude_this_frame {
            self.amplitude_this_frame = amplitude;
        }
    }
}
