use std::sync::Arc;

use oddio::{Frames, Stop};

type AudioHandle = oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::FramesSignal<f32>>>>;

pub struct AudioSource {
    pub frames: Arc<Frames<f32>>,
    pub handle: Option<AudioHandle>,
    pub next_state: AudioState,
}

#[derive(Debug, Clone, Copy)]
pub enum AudioState {
    Stopped,
    Playing,
    Paused,
}

impl AudioSource {
    pub fn new(frames: Arc<Frames<f32>>) -> Self {
        Self {
            frames,
            handle: None,
            next_state: AudioState::Stopped,
        }
    }

    pub fn current_state(&mut self) -> AudioState {
        if let Some(handle) = self.handle.as_mut() {
            let control = handle.control::<Stop<_>, _>();
            if control.is_paused() {
                return AudioState::Paused;
            }
            if control.is_stopped() {
                return AudioState::Stopped;
            }
            return AudioState::Playing;
        } else {
            return AudioState::Stopped;
        }
    }

    pub fn play(&mut self) {
        self.next_state = AudioState::Playing;
    }

    pub fn pause(&mut self) {
        self.next_state = AudioState::Paused;
    }

    pub fn stop(&mut self) {
        self.next_state = AudioState::Stopped;
    }

    pub fn resume(&mut self) {
        self.next_state = AudioState::Playing;
    }
}
