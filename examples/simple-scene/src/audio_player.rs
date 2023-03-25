use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hotham::anyhow::{self, anyhow};
use nalgebra::{self, DMatrix, DVector, Isometry3, Point3, Unit, Vector3};
use triple_buffer as tbuf;

pub type SamplingFunction = Box<dyn Send + FnMut(bool, &Point3<f32>) -> f32>;
pub type ListenerPose = Isometry3<f32>;

pub struct AudioPlayer {
    device: cpal::Device,
    pub config: cpal::SupportedStreamConfig,
    stream: Option<anyhow::Result<cpal::Stream>>,
    global_from_listener: ListenerPose,
    pub enable_band_pass_filter: Arc<AtomicBool>,
    pub num_frames_per_callback: Arc<AtomicUsize>,
    ring_buffers: Option<RingBuffersNonRealTimeSides>,
}

struct RingBuffersNonRealTimeSides {
    global_from_listener_producer: rtrb::Producer<ListenerPose>,
    sampling_function_producer: tbuf::Input<Option<SamplingFunction>>,
    to_ui_consumer: rtrb::Consumer<(f32, f32, f32, f32)>,
}

struct RingBuffersRealTimeSides {
    global_from_listener_consumer: rtrb::Consumer<ListenerPose>,
    sampling_function_consumer: tbuf::Output<Option<SamplingFunction>>,
    to_ui_producer: rtrb::Producer<(f32, f32, f32, f32)>,
}

impl AudioPlayer {
    pub fn new() -> anyhow::Result<AudioPlayer> {
        puffin::profile_function!();
        let host = cpal::default_host();

        let optional_device = host.default_output_device();
        if optional_device.is_none() {
            anyhow::bail!("No output device is available");
        }
        let device = optional_device.unwrap();
        println!("Output device: {}", device.name()?);

        let config = device.default_output_config()?;
        println!("Default output config: {:?}", config);

        let mut audio_player = AudioPlayer {
            device,
            config,
            stream: None,
            ring_buffers: None,
            global_from_listener: ListenerPose::face_towards(
                &Point3::new(0.0, 1.0, 0.0),
                &Point3::new(0.0, 1.0, 1.0),
                &Vector3::new(0.0, 1.0, 0.0),
            ),
            enable_band_pass_filter: Arc::new(AtomicBool::new(true)),
            num_frames_per_callback: Arc::new(AtomicUsize::new(0)),
        };
        audio_player.start_output_stream()?;
        Ok(audio_player)
    }

    fn start_output_stream(&mut self) -> anyhow::Result<()> {
        puffin::profile_function!();
        let (sampling_function_producer, sampling_function_consumer) =
            tbuf::TripleBuffer::<Option<SamplingFunction>>::default().split();
        // let (disposal_queue_producer, disposal_queue_consumer) = rtrb::RingBuffer::new(2);
        // let (sampling_function_producer, sampling_function_consumer) = rtrb::RingBuffer::new(2);
        let (mut global_from_listener_producer, global_from_listener_consumer) =
            rtrb::RingBuffer::new(100);
        global_from_listener_producer
            .push(self.global_from_listener)
            .unwrap(); // Initialize listener position
        let (to_ui_producer, to_ui_consumer) =
            rtrb::RingBuffer::new(self.config.sample_rate().0 as usize);
        self.ring_buffers = Some(RingBuffersNonRealTimeSides {
            sampling_function_producer,
            global_from_listener_producer,
            to_ui_consumer,
        });
        let realtime_sides = RingBuffersRealTimeSides {
            global_from_listener_consumer,
            sampling_function_consumer,
            to_ui_producer,
        };
        self.stream = Some(match self.config.sample_format() {
            cpal::SampleFormat::F32 => run::<f32>(
                &self.device,
                &self.config.clone().into(),
                self.enable_band_pass_filter.clone(),
                self.num_frames_per_callback.clone(),
                realtime_sides,
                self.global_from_listener,
            ),
            cpal::SampleFormat::I16 => run::<i16>(
                &self.device,
                &self.config.clone().into(),
                self.enable_band_pass_filter.clone(),
                self.num_frames_per_callback.clone(),
                realtime_sides,
                self.global_from_listener,
            ),
            cpal::SampleFormat::U16 => run::<u16>(
                &self.device,
                &self.config.clone().into(),
                self.enable_band_pass_filter.clone(),
                self.num_frames_per_callback.clone(),
                realtime_sides,
                self.global_from_listener,
            ),
        });
        Ok(())
    }

    pub fn play_audio(&mut self, next_sample: SamplingFunction) -> anyhow::Result<()> {
        puffin::profile_function!();
        if let Some(RingBuffersNonRealTimeSides {
            sampling_function_producer,
            ..
        }) = &mut self.ring_buffers
        {
            sampling_function_producer.write(Some(next_sample));
            Ok(())
        } else {
            Err(anyhow!("Audio not initialized"))
        }
    }

    pub fn set_listener_pose(&mut self, global_from_listener: &ListenerPose) -> anyhow::Result<()> {
        self.global_from_listener = *global_from_listener;
        if let Some(RingBuffersNonRealTimeSides {
            global_from_listener_producer,
            ..
        }) = &mut self.ring_buffers
        {
            global_from_listener_producer.push(self.global_from_listener)?;
        }
        Ok(())
    }

    pub fn get_audio_history_entry(&mut self) -> Option<(f32, f32, f32, f32)> {
        if let Some(RingBuffersNonRealTimeSides { to_ui_consumer, .. }) = &mut self.ring_buffers {
            to_ui_consumer.pop().ok()
        } else {
            None
        }
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    enable_band_pass_filter: Arc<AtomicBool>,
    num_frames_per_callback: Arc<AtomicUsize>,
    RingBuffersRealTimeSides {
        mut sampling_function_consumer,
        mut global_from_listener_consumer,
        mut to_ui_producer,
    }: RingBuffersRealTimeSides,
    listener_pos: ListenerPose,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample,
{
    puffin::profile_function!();
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

    const SPEED_OF_SOUND: f32 = 343.0;
    const PLAYBACK_HISTORY_SIZE: usize = 5000;
    let mut playback_history = DMatrix::<f32>::zeros(PLAYBACK_HISTORY_SIZE, channels);
    let mut impulse_response = DVector::<f32>::zeros(PLAYBACK_HISTORY_SIZE);
    let impulse_size = (config.sample_rate.0 as f32 * 0.5 / SPEED_OF_SOUND) as usize;
    impulse_response
        .rows_mut(PLAYBACK_HISTORY_SIZE - 1 - impulse_size, impulse_size)
        .fill(1.0 / impulse_size as f32);
    // let player_sample_rate = config.sample_rate.0 as f32;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1] = 1.0;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.002) as usize] = 0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.004) as usize] = 0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.006) as usize] = 0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.0075) as usize] = -0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.009) as usize] = 0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.011) as usize] = -0.5;
    // impulse_response[PLAYBACK_HISTORY_SIZE - 1 - (player_sample_rate * 0.013) as usize] = -0.5;
    let impulse_response = impulse_response;
    let mut playback_write_index = 0;

    // Exponential moving average band-pass filtering
    const ALPHA1: f32 = 0.01;
    const ALPHA2: f32 = 0.001;
    const ALPHA_ATTACK: f32 = 1.0;
    const ALPHA_RELEASE: f32 = 0.0001;
    const BASELINE: f32 = 100.0;
    const HEADROOM_FRACTION: f32 = 0.25;
    const HEADROOM_FACTOR: f32 = 1.0 - HEADROOM_FRACTION;
    let mut moving_power_average_filtered = BASELINE;
    let mut moving_power_average_raw = BASELINE;
    let mut global_from_listener_after = listener_pos;

    let offsets_by_channel: Vec<Point3<f32>> = match channels {
        1 => vec![Point3::new(0.0, 0.0, 0.0)],
        2 => vec![Point3::new(-0.1, 0.0, 0.0), Point3::new(0.1, 0.0, 0.0)],
        _ => panic!("Cannot handle {} channels", channels),
    };
    let mut sample_by_channel = vec![0.0; channels];
    let mut filtered_sample_by_channel = vec![0.0; channels];
    let mut moving_average1 = vec![0_f32; channels];
    let mut moving_average2 = vec![0_f32; channels];

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            puffin::profile_scope!("data_callback");
            let global_from_listener_before = global_from_listener_after;
            while let Ok(new_global_from_listener) = global_from_listener_consumer.pop() {
                global_from_listener_after = new_global_from_listener;
            }

            // Report num_frames_per_callback
            let num_frames = data.len() / channels;
            num_frames_per_callback.store(num_frames, Ordering::Relaxed);

            // Manually fetch the buffer update because we are using output_buffer() instead of read()
            sampling_function_consumer.update();

            // Acquire a mutable reference to the output buffer so that we can call the sampling function
            if let Some(sampling_function) = &mut sampling_function_consumer.output_buffer() {
                for (i, frame) in data.chunks_mut(channels).enumerate() {
                    // Interpolate listener position
                    let t = i as f32 / num_frames as f32;
                    let global_from_listener = ListenerPose::from_parts(
                        (global_from_listener_before.translation.vector * (1.0 - t)
                            + global_from_listener_after.translation.vector * t)
                            .into(),
                        Unit::new_normalize(
                            global_from_listener_before
                                .rotation
                                .lerp(&global_from_listener_after.rotation, t),
                        ),
                    );

                    // Sample each channel with offset on listener position
                    for channel in 0..channels {
                        sample_by_channel[channel] = {
                            let sample = sampling_function(
                                channel == 0,
                                &global_from_listener.transform_point(&offsets_by_channel[channel]),
                            );
                            if sample.is_finite() {
                                sample
                            } else {
                                0_f32
                            }
                        };
                    }
                    {
                        puffin::profile_scope!("filter_audio");
                        // Convolve with impulse response
                        for channel in 0..channels {
                            playback_history[(playback_write_index, channel)] =
                                sample_by_channel[channel];
                        }
                        playback_write_index = (playback_write_index + 1) % PLAYBACK_HISTORY_SIZE;
                        for channel in 0..channels {
                            let nrows = PLAYBACK_HISTORY_SIZE - playback_write_index;
                            sample_by_channel[channel] = playback_history
                                .column(channel)
                                .rows(playback_write_index, nrows)
                                .dot(&impulse_response.rows(0, nrows))
                                + playback_history
                                    .column(channel)
                                    .rows(0, playback_write_index)
                                    .dot(&impulse_response.rows(nrows, playback_write_index))
                        }

                        // Band-pass filter
                        for channel in 0..channels {
                            let value = sample_by_channel[channel];
                            moving_average1[channel] =
                                moving_average1[channel] * (1.0 - ALPHA1) + value * ALPHA1;
                            moving_average2[channel] =
                                moving_average2[channel] * (1.0 - ALPHA2) + value * ALPHA2;
                            filtered_sample_by_channel[channel] =
                                moving_average1[channel] - moving_average2[channel];
                        }
                    }
                    puffin::profile_scope!("normalize_audio");
                    // Adjust volume jointly over raw samples
                    let raw_tall_puppy = sample_by_channel
                        .iter()
                        .map(|x| x.abs())
                        .fold(BASELINE, f32::max);
                    if raw_tall_puppy > moving_power_average_raw {
                        moving_power_average_raw = moving_power_average_raw * (1.0 - ALPHA_ATTACK)
                            + raw_tall_puppy * ALPHA_ATTACK;
                    } else {
                        moving_power_average_raw = moving_power_average_raw * (1.0 - ALPHA_RELEASE)
                            + raw_tall_puppy * ALPHA_RELEASE;
                    }
                    let normalization = HEADROOM_FACTOR / moving_power_average_raw;

                    // Adjust volume jointly over filtered samples
                    let filtered_tall_puppy = filtered_sample_by_channel
                        .iter()
                        .map(|x| x.abs())
                        .fold(BASELINE, f32::max);
                    if filtered_tall_puppy > moving_power_average_filtered {
                        moving_power_average_filtered = moving_power_average_filtered
                            * (1.0 - ALPHA_ATTACK)
                            + filtered_tall_puppy * ALPHA_ATTACK;
                    } else {
                        moving_power_average_filtered = moving_power_average_filtered
                            * (1.0 - ALPHA_RELEASE)
                            + filtered_tall_puppy * ALPHA_RELEASE;
                    }
                    let normalization_filtered = HEADROOM_FACTOR / moving_power_average_filtered;

                    // Try to push but ignore if it works or not.
                    let _ = to_ui_producer.push((
                        sample_by_channel[0],
                        if channels >= 2 {
                            sample_by_channel[1]
                        } else {
                            0.0
                        },
                        moving_power_average_raw,
                        moving_power_average_filtered,
                    ));

                    for sample in &mut sample_by_channel {
                        *sample *= normalization;
                    }
                    for sample in &mut filtered_sample_by_channel {
                        *sample *= normalization_filtered;
                    }

                    if enable_band_pass_filter.load(Ordering::Relaxed) {
                        for (channel, sample) in frame.iter_mut().enumerate() {
                            *sample =
                                cpal::Sample::from::<f32>(&filtered_sample_by_channel[channel]);
                        }
                    } else {
                        for (channel, sample) in frame.iter_mut().enumerate() {
                            *sample = cpal::Sample::from::<f32>(&sample_by_channel[channel]);
                        }
                    }
                }
            } else {
                data.fill(cpal::Sample::from::<f32>(&0.0));
            }
        },
        err_fn,
    )?;
    stream.play()?;
    Ok(stream)
}
