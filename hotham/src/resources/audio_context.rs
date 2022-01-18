use std::sync::{Arc, Mutex};

use crate::components::AudioSource;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleRate, Stream,
};
use oddio::{Mixer, SpatialScene};
use symphonia::core::{audio::SampleBuffer, io::MediaSourceStream};
use symphonia::core::{codecs::Decoder, probe::Hint};

pub struct AudioContext {
    // pub scene_handle: oddio::Handle<SpatialScene>,
    pub mixer_handle: oddio::Handle<Mixer<[f32; 2]>>,
    pub sample_rate: SampleRate,
    pub stream: Arc<Mutex<Stream>>,
}

impl Default for AudioContext {
    fn default() -> Self {
        // Configure cpal
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        println!("Detected default audio device: {}", device.name().unwrap());
        let sample_rate = device.default_output_config().unwrap().sample_rate();
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };
        println!("Config: {:?}", config);

        // Produce a sinusoid of maximum amplitude.
        let sr = config.sample_rate.0 as f32;
        let mut sample_clock = 0f32;
        let mut next_value = move || {
            sample_clock = (sample_clock + 1.0) % sr;
            (sample_clock * 440.0 * 2.0 * std::f32::consts::PI / sr).sin()
        };

        // let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new(sample_rate.0, 0.1));

        let channels = config.channels as usize;
        let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

        // let config = device.default_output_config().unwrap();
        // let stream = device
        //     .build_output_stream(
        //         &config.into(),
        //         move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
        //             for frame in data.chunks_mut(channels) {
        //                 let value = cpal::Sample::from::<f32>(&next_value());
        //                 for sample in frame.iter_mut() {
        //                     *sample = value;
        //                 }
        //             }
        //         },
        //         err_fn,
        //     )
        //     .expect("Unable to create stream");
        // stream.play().expect("Unable to play stream");
        let (mut mixer_handle, mixer) = oddio::split(oddio::Mixer::new());

        let stream = device
            .build_output_stream(
                &config,
                move |out_flat: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let out_stereo: &mut [[f32; 2]] = oddio::frame_stereo(out_flat);
                    oddio::run(&mixer, sample_rate.0, out_stereo);
                },
                err_fn,
            )
            .unwrap();
        stream.play().expect("Unable to play stream!");

        Self {
            // scene_handle,
            mixer_handle,
            sample_rate,
            stream: Arc::new(Mutex::new(stream)),
        }
    }
}

// SAFETY: I solemly promise to be good.
// We have no intention to mutate `Stream`, we just have to hold
// a reference onto it so that sound keeps playing.
unsafe impl Send for AudioContext {}

impl AudioContext {
    pub fn create_audio_source(&mut self, mp3_bytes: Vec<u8>) -> AudioSource {
        let mut decoded = decode_mp3_data(mp3_bytes);
        let stereo = oddio::frame_stereo(&mut decoded);
        let frames = oddio::Frames::from_slice(44100, &stereo);
        // let frames = oddio::Frames::from_iter(self.sample_rate.0, decoded);

        // let basic_signal: oddio::FramesSignal<_> = oddio::FramesSignal::from(boop);
        let basic_signal: oddio::FramesSignal<_> = oddio::FramesSignal::from(frames);
        // let gain = oddio::Gain::new(basic_signal, 1.0);

        let handle = self
            // .scene_handle
            .mixer_handle
            // .control::<oddio::SpatialScene, _>()
            .control()
            .play(basic_signal);
        // let handle = self
        //     .scene_handle
        //     .control::<oddio::SpatialScene, _>()
        //     .play_buffered(
        //         gain,
        //         oddio::SpatialOptions {
        //             position: [0., 0., -1.0].into(),
        //             velocity: [0., 0.0, 0.0].into(),
        //             radius: 0.0,
        //         },
        //         1000.0,
        //     );
        AudioSource { handle }
    }
}

fn interleave(frames: Vec<f32>) -> Vec<[f32; 2]> {
    let mut result = Vec::new();
    for slice in frames.windows(2) {
        result.push([slice[0], slice[1]]);
    }

    result
}

fn decode_mp3_data(mp3_bytes: Vec<u8>) -> Vec<f32> {
    let cursor = Box::new(std::io::Cursor::new(mp3_bytes));
    let mss = MediaSourceStream::new(cursor, Default::default());
    let hint = Hint::new();
    let format_opts = Default::default();
    let metadata_opts = Default::default();
    let decode_opts = Default::default();
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &metadata_opts)
        .expect("Failed to parse MP3 file");

    let mut reader = probed.format;
    let track = reader.default_track().unwrap();
    let track_id = track.id;
    let mut decoder = symphonia::default::get_codecs()
        .make(&track.codec_params, &decode_opts)
        .expect("Unable to get decoder");
    show_decoder_info(&decoder);

    let mut samples: Vec<f32> = Vec::new();

    // Decode all packets, ignoring all decode errors.
    loop {
        let packet = match reader.next_packet() {
            Ok(packet) => packet,
            Err(err) => {
                eprintln!("Error reading packet: {:?}", err);
                break;
            }
        };

        // If the packet does not belong to the selected track, skip over it.
        if packet.track_id() != track_id {
            continue;
        }

        // Decode the packet into audio samples.
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let mut sample_buf =
                    SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
                sample_buf.copy_interleaved_ref(decoded);
                for sample in sample_buf.samples() {
                    samples.push(*sample);
                }
            }
            Err(err) => {
                eprintln!("Error while decoding: {:?}", err);
                break;
            }
        }
    }

    // Regardless of result, finalize the decoder to get the verification result.
    samples
}

fn show_decoder_info(decoder: &Box<dyn Decoder>) {
    let sample_rate = decoder.codec_params().sample_rate;
    let channels = decoder.codec_params().channels.unwrap().count();
    println!("Sample rate: {:?}", sample_rate);
    println!("Channels: {:?}", channels);
}
