use std::sync::Arc;

use oddio::{Frames, Stop};

type AudioHandle = oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::FramesSignal<f32>>>>;

/// A component added to an entity to allow it to emit a sound, usually a sound effect
/// Used by `audio_system`
pub struct SoundEmitter {
    /// The actual sound data
    pub frames: Arc<Frames<f32>>,
    /// Handle into the `oddio` spatialiser
    pub handle: Option<AudioHandle>,
    /// Used to indicate that the emitter wants to change its state
    pub next_state: Option<SoundState>,
}

impl Clone for SoundEmitter {
    fn clone(&self) -> Self {
        Self {
            frames: self.frames.clone(),
            handle: None,
            next_state: None,
        }
    }
}

/// State of a sound
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundState {
    /// The sound has stopped permenantly
    Stopped,
    /// The sound is playing
    Playing,
    /// The sound is temporarily paused
    Paused,
}

impl SoundEmitter {
    /// Convenience function to create a new `SoundEmitter`
    pub fn new(frames: Arc<Frames<f32>>) -> Self {
        Self {
            frames,
            handle: None,
            next_state: None,
        }
    }

    /// Convenience function to get the `SoundState` of this `SoundEmitter`
    pub fn current_state(&mut self) -> SoundState {
        if let Some(handle) = self.handle.as_mut() {
            let control = handle.control::<Stop<_>, _>();
            if control.is_paused() {
                return SoundState::Paused;
            }
            if control.is_stopped() {
                return SoundState::Stopped;
            }
            SoundState::Playing
        } else {
            SoundState::Stopped
        }
    }

    /// Play the sound
    pub fn play(&mut self) {
        self.next_state = Some(SoundState::Playing);
    }

    /// Pause the sound
    pub fn pause(&mut self) {
        self.next_state = Some(SoundState::Paused);
    }

    /// Stop the sound
    pub fn stop(&mut self) {
        self.next_state = Some(SoundState::Stopped);
    }

    /// Resume the sound
    pub fn resume(&mut self) {
        self.next_state = Some(SoundState::Playing);
    }
}
