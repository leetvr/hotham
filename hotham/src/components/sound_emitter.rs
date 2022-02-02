use std::sync::Arc;

use oddio::{Frames, Stop};

type AudioHandle = oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::FramesSignal<f32>>>>;

pub struct SoundEmitter {
    pub frames: Arc<Frames<f32>>,
    pub handle: Option<AudioHandle>,
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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundState {
    Stopped,
    Playing,
    Paused,
}

impl SoundEmitter {
    pub fn new(frames: Arc<Frames<f32>>) -> Self {
        Self {
            frames,
            handle: None,
            next_state: None,
        }
    }

    pub fn current_state(&mut self) -> SoundState {
        if let Some(handle) = self.handle.as_mut() {
            let control = handle.control::<Stop<_>, _>();
            if control.is_paused() {
                return SoundState::Paused;
            }
            if control.is_stopped() {
                return SoundState::Stopped;
            }
            return SoundState::Playing;
        } else {
            return SoundState::Stopped;
        }
    }

    pub fn play(&mut self) {
        self.next_state = Some(SoundState::Playing);
    }

    pub fn pause(&mut self) {
        self.next_state = Some(SoundState::Paused);
    }

    pub fn stop(&mut self) {
        self.next_state = Some(SoundState::Stopped);
    }

    pub fn resume(&mut self) {
        self.next_state = Some(SoundState::Playing);
    }
}
