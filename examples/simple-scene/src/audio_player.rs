use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hotham::anyhow::{self, anyhow};
use nalgebra::{self, Point3, Vector3};
use triple_buffer as tbuf;

type SamplingFunction = Box<dyn Send + FnMut(bool, ListenerPos) -> f32>;
type ListenerPos = Point3<f32>;

pub struct AudioPlayer {
    device: cpal::Device,
    pub config: cpal::SupportedStreamConfig,
    stream: Option<anyhow::Result<cpal::Stream>>,
    listener_pos: ListenerPos,
    pub enable_band_pass_filter: Arc<AtomicBool>,
    pub num_frames_per_callback: Arc<AtomicUsize>,
    ring_buffers: Option<RingBuffersNonRealTimeSides>,
}

struct RingBuffersNonRealTimeSides {
    listener_pos_producer: rtrb::Producer<ListenerPos>,
    sampling_function_producer: tbuf::Input<Option<SamplingFunction>>,
    to_ui_consumer: rtrb::Consumer<(f32, f32, f32, f32)>,
}

struct RingBuffersRealTimeSides {
    listener_pos_consumer: rtrb::Consumer<ListenerPos>,
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
            listener_pos: Point3::new(0.0, 1.0, 0.0),
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
        let (mut listener_pos_producer, listener_pos_consumer) = rtrb::RingBuffer::new(100);
        listener_pos_producer.push(self.listener_pos).unwrap(); // Initialize listener position
        let (to_ui_producer, to_ui_consumer) =
            rtrb::RingBuffer::new(self.config.sample_rate().0 as usize);
        self.ring_buffers = Some(RingBuffersNonRealTimeSides {
            sampling_function_producer,
            listener_pos_producer,
            to_ui_consumer,
        });
        let realtime_sides = RingBuffersRealTimeSides {
            listener_pos_consumer,
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
                self.listener_pos,
            ),
            cpal::SampleFormat::I16 => run::<i16>(
                &self.device,
                &self.config.clone().into(),
                self.enable_band_pass_filter.clone(),
                self.num_frames_per_callback.clone(),
                realtime_sides,
                self.listener_pos,
            ),
            cpal::SampleFormat::U16 => run::<u16>(
                &self.device,
                &self.config.clone().into(),
                self.enable_band_pass_filter.clone(),
                self.num_frames_per_callback.clone(),
                realtime_sides,
                self.listener_pos,
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

    pub fn set_listener_pos(&mut self, listener_pos: ListenerPos) -> anyhow::Result<()> {
        self.listener_pos = listener_pos;
        if let Some(RingBuffersNonRealTimeSides {
            listener_pos_producer,
            ..
        }) = &mut self.ring_buffers
        {
            listener_pos_producer.push(self.listener_pos)?;
        }
        Ok(())
    }
}

fn run<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    enable_band_pass_filter: Arc<AtomicBool>,
    num_frames_per_callback: Arc<AtomicUsize>,
    RingBuffersRealTimeSides {
        mut sampling_function_consumer,
        mut listener_pos_consumer,
        mut to_ui_producer,
    }: RingBuffersRealTimeSides,
    listener_pos: ListenerPos,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample,
{
    puffin::profile_function!();
    let channels = config.channels as usize;
    let err_fn = |err| eprintln!("an error occurred on stream: {}", err);

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
    let mut listener_pos_after = listener_pos;

    let offsets_by_channel = match channels {
        1 => vec![Vector3::zeros()],
        2 => vec![Vector3::new(-0.1, 0.0, 0.0), Vector3::new(0.1, 0.0, 0.0)],
        _ => panic!("Cannot handle {} channels", channels),
    };
    let mut sample_by_channel = vec![0.0; channels];
    let mut filtered_sample_by_channel = vec![0.0; channels];
    let mut moving_average1 = vec![0_f32; channels];
    let mut moving_average2 = vec![0_f32; channels];

    let stream = device.build_output_stream(
        config,
        move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
            puffin::profile_function!();
            let listener_pos_before = listener_pos_after;
            while let Ok(new_listener_pos) = listener_pos_consumer.pop() {
                listener_pos_after = new_listener_pos;
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
                    let listener_pos =
                        listener_pos_before * (1.0 - t) + listener_pos_after.coords * t;

                    // Sample each channel with offset on listener position
                    for channel in 0..channels {
                        sample_by_channel[channel] = {
                            let sample = sampling_function(
                                channel == 0,
                                listener_pos + offsets_by_channel[channel],
                            );
                            if sample.is_finite() {
                                sample
                            } else {
                                0_f32
                            }
                        };
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

                    puffin::profile_scope!("filter_audio");
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
