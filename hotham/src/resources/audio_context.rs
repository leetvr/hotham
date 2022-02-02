use std::sync::{Arc, Mutex};

use crate::components::{sound_emitter::SoundState, SoundEmitter};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Stream,
};
use oddio::{Frames, FramesSignal, Handle, Mixer, SpatialBuffered, SpatialScene, Stop};
use symphonia::core::{audio::SampleBuffer, io::MediaSourceStream, probe::Hint};

type MusicTrackHandle = Handle<Stop<FramesSignal<[f32; 2]>>>;
use generational_arena::{Arena, Index};

pub struct AudioContext {
    pub scene_handle: oddio::Handle<SpatialScene>,
    pub mixer_handle: oddio::Handle<Mixer<[f32; 2]>>,
    pub stream: Arc<Mutex<Stream>>,
    pub current_music_track: Option<MusicTrack>,
    music_tracks_inner: Arena<Arc<Frames<[f32; 2]>>>,
    music_track_handle: Option<MusicTrackHandle>,
}

#[derive(Debug, Clone, PartialEq, Copy)]
pub struct MusicTrack {
    index: Index,
}

impl Default for AudioContext {
    fn default() -> Self {
        // Configure cpal
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .expect("no output device available");
        println!(
            "[HOTHAM_AUDIO_CONTEXT] Using default audio device: {}",
            device.name().unwrap()
        );
        let sample_rate = device.default_output_config().unwrap().sample_rate();
        let config = cpal::StreamConfig {
            channels: 2,
            sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };
        println!("[HOTHAM_AUDIO_CONTEXT] cpal AudioConfig: {:?}", config);

        // Create a spatialised audio scene
        let (scene_handle, scene) = oddio::split(oddio::SpatialScene::new(sample_rate.0, 0.1));

        // Create a mixer
        let (mut mixer_handle, mixer) = oddio::split(oddio::Mixer::new());

        // Pipe the spatialised scene to the mixer
        let _ = mixer_handle.control().play(scene);

        // Pipe the mixer to the audio hardware.
        let stream = device
            .build_output_stream(
                &config,
                move |out_flat: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    let out_stereo: &mut [[f32; 2]] = oddio::frame_stereo(out_flat);
                    oddio::run(&mixer, sample_rate.0, out_stereo);
                },
                |err| {
                    eprintln!(
                        "[HOTHAM_AUDIO_CONTEXT] An error occurred playing the audio stream: {}",
                        err
                    )
                },
            )
            .unwrap();
        stream
            .play()
            .expect("[HOTHAM_AUDIO_CONTEXT] Unable to play to audio hardware!");

        Self {
            scene_handle,
            mixer_handle,
            stream: Arc::new(Mutex::new(stream)),
            music_tracks_inner: Arena::new(),
            music_track_handle: None,
            current_music_track: None,
        }
    }
}

// SAFETY: I solemly promise to be good.
// We have no intention to mutate `Stream`, we just have to hold
// a reference onto it so that sound keeps playing.
unsafe impl Send for AudioContext {}

impl AudioContext {
    pub fn create_audio_source(&mut self, mp3_bytes: Vec<u8>) -> SoundEmitter {
        let frames = get_frames_from_mp3(mp3_bytes);

        SoundEmitter::new(frames)
    }

    pub fn play_audio(
        &mut self,
        sound_emitter: &mut SoundEmitter,
        position: mint::Point3<f32>,
        velocity: mint::Vector3<f32>,
    ) {
        let signal: oddio::FramesSignal<_> =
            oddio::FramesSignal::from(sound_emitter.frames.clone());
        let handle = self.scene_handle.control().play_buffered(
            signal,
            oddio::SpatialOptions {
                position,
                velocity,
                radius: 1.0, //
            },
            1000.0,
        );
        sound_emitter.handle = Some(handle);
    }

    pub fn resume_audio(&mut self, audio_source: &mut SoundEmitter) {
        audio_source
            .handle
            .as_mut()
            .map(|h| h.control::<Stop<_>, _>().resume());
    }

    pub fn pause_audio(&mut self, audio_source: &mut SoundEmitter) {
        audio_source
            .handle
            .as_mut()
            .map(|h| h.control::<Stop<_>, _>().pause());
    }

    pub fn stop_audio(&mut self, audio_source: &mut SoundEmitter) {
        audio_source
            .handle
            .take()
            .map(|mut h| h.control::<Stop<_>, _>().stop());
    }

    pub fn update_motion(
        &mut self,
        audio_source: &mut SoundEmitter,
        position: mint::Point3<f32>,
        velocity: mint::Vector3<f32>,
    ) {
        audio_source.handle.as_mut().map(|h| {
            h.control::<SpatialBuffered<_>, _>()
                .set_motion(position, velocity, false)
        });
    }

    pub fn add_music_track(&mut self, mp3_bytes: Vec<u8>) -> MusicTrack {
        println!("[AUDIO_CONTEXT] Decoding MP3..");
        let frames = get_stereo_frames_from_mp3(mp3_bytes);
        println!("[AUDIO_CONTEXT] ..done!");
        let track = MusicTrack {
            index: self.music_tracks_inner.insert(frames),
        };
        track
    }

    pub fn play_music_track(&mut self, track: MusicTrack) {
        if let Some(mut handle) = self.music_track_handle.take() {
            handle.control::<Stop<_>, _>().stop();
        }

        let frames = self.music_tracks_inner[track.index].clone();
        let signal = oddio::FramesSignal::from(frames);
        self.music_track_handle = Some(self.mixer_handle.control().play(signal));
        self.current_music_track = Some(track.clone());
    }

    pub fn pause_music_track(&mut self) {
        self.music_track_handle
            .as_mut()
            .map(|h| h.control::<Stop<_>, _>().pause());
    }

    pub fn resume_music_track(&mut self) {
        self.music_track_handle
            .as_mut()
            .map(|h| h.control::<Stop<_>, _>().resume());
    }

    pub fn music_track_status(&mut self) -> SoundState {
        if let Some(handle) = self.music_track_handle.as_mut() {
            let control = handle.control::<Stop<_>, _>();
            if control.is_paused() {
                return SoundState::Paused;
            }
            if control.is_stopped() {
                return SoundState::Stopped;
            }
            return SoundState::Playing;
        } else {
            SoundState::Stopped
        }
    }

    pub fn dummy_track(&mut self) -> MusicTrack {
        let frames = oddio::Frames::from_slice(0, &[]);
        MusicTrack {
            index: self.music_tracks_inner.insert(frames),
        }
    }

    pub fn dummy_sound_emitter(&mut self) -> SoundEmitter {
        let frames = oddio::Frames::from_slice(0, &[]);
        SoundEmitter::new(frames)
    }
}

fn get_frames_from_mp3(mp3_bytes: Vec<u8>) -> Arc<Frames<f32>> {
    let (samples, sample_rate) = decode_mp3(mp3_bytes);
    oddio::Frames::from_slice(sample_rate, &samples)
}

fn get_stereo_frames_from_mp3(mp3_bytes: Vec<u8>) -> Arc<Frames<[f32; 2]>> {
    let (mut samples, sample_rate) = decode_mp3(mp3_bytes);
    let stereo = oddio::frame_stereo(&mut samples);
    oddio::Frames::from_slice(sample_rate, &stereo)
}

fn decode_mp3(mp3_bytes: Vec<u8>) -> (Vec<f32>, u32) {
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
    let sample_rate = decoder.codec_params().sample_rate.unwrap();

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

    (samples, sample_rate)
}
