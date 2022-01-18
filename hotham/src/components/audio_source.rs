use oddio::{FramesSignal, Handle, MonoToStereo, Sine, Stop};

// type AudioHandle =
//     oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::Gain<oddio::FramesSignal<f32>>>>>;

// type AudioHandle = Handle<Stop<MonoToStereo<Sine>>>;
// type AudioHandle = oddio::Handle<Stop<MonoToStereo<FramesSignal<f32>>>>;
type AudioHandle = oddio::Handle<Stop<FramesSignal<[f32; 2]>>>;

pub struct AudioSource {
    pub handle: AudioHandle,
}
