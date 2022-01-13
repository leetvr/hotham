type AudioHandle =
    oddio::Handle<oddio::SpatialBuffered<oddio::Stop<oddio::Gain<oddio::FramesSignal<f32>>>>>;

pub struct AudioSource {
    pub handle: AudioHandle,
}
