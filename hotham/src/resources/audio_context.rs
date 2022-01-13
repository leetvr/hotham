use crate::components::AudioSource;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate,
};
use oddio::SpatialScene;
use symphonia::core::io::{BitReaderRtl, MediaSourceStream, ReadOnlySource};

pub struct AudioContext {
    pub scene_handle: oddio::Handle<SpatialScene>,
    pub sample_rate: SampleRate,
}

impl Default for AudioContext {
    fn default() -> Self {
        // Configure cpal
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        let sample_rate = device.default_output_config().unwrap().sample_rate();
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new(sample_rate.0, 0.1));

        let stream = device
            .build_output_stream(
                &config,
                move |out_flat: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let out_stereo: &mut [[f32; 2]] = oddio::frame_stereo(out_flat);
                    oddio::run(&scene, sample_rate.0, out_stereo);
                },
                move |err| {
                    eprintln!("{}", err);
                },
            )
            .unwrap();
        stream.play().unwrap();

        Self {
            scene_handle,
            sample_rate,
        }
    }
}

impl AudioContext {
    pub fn create_audio_source(&mut self, mp3_bytes: &[u8]) -> AudioSource {
        let cursor = Box::new(std::io::Cursor::new(mp3_bytes));
        let mss = MediaSourceStream::new(cursor, Default::default());
        let decoded = todo!();
        let frames = oddio::Frames::from_iter(self.sample_rate.0, decoded);
        let basic_signal: oddio::FramesSignal<_> = oddio::FramesSignal::from(frames);
        let gain = oddio::Gain::new(basic_signal, 1.0);

        let mut handle = self
            .scene_handle
            .control::<oddio::SpatialScene, _>()
            .play_buffered(
                gain,
                oddio::SpatialOptions {
                    position: [0., 10.0, 0.0].into(),
                    velocity: [0., 0.0, 0.0].into(),
                    radius: 0.1,
                },
                1000.0,
            );
        AudioSource {}
    }
}
