use std::sync::{Arc, Mutex};

use hotham::anyhow;
use nalgebra::{self, Affine3, DMatrix, DVector, Point3, Scale3, Translation3, Vector3};

use crate::audio_player::AudioPlayer;

pub struct AudioState {
    pub listener_pos: Point3<f32>,
    pub audio_player: AudioPlayer,
    pub audio_history: Vec<(f32, f32, f32, f32)>,
    pub audio_history_index: usize,
    pub audio_history_resolution: usize,
    pub to_audio_producer: rtrb::Producer<DVector<f32>>,
    pub to_ui_consumer: rtrb::Consumer<DVector<f32>>,
}

impl AudioState {
    pub fn init_audio(num_points: usize) -> anyhow::Result<AudioState> {
        puffin::profile_function!();
        let (to_audio_producer, mut to_audio_consumer) = rtrb::RingBuffer::new(100);
        let (mut to_ui_producer, to_ui_consumer) = rtrb::RingBuffer::new(100);

        let mut audio_player = AudioPlayer::new()?;
        let player_sample_rate = audio_player.config.sample_rate().0 as f32;

        // Keep a history of states in a circular buffer so that we can create a waveform by combining contributions over time.
        const SAMPLES_IN_BUFFER: usize = 1024;
        const SPEED_OF_SOUND: f32 = 343.0;
        const SIMULATION_RATE: f32 = 1000.0;
        let mut player_samples_per_simulation_state = player_sample_rate / SIMULATION_RATE;
        let mut state_history = DMatrix::<f32>::zeros(num_points * 3 * 2, SAMPLES_IN_BUFFER);
        let mut acc_history = DMatrix::<f32>::zeros(num_points * 3, SAMPLES_IN_BUFFER);
        let mut index_of_newest: usize = 0;
        let mut num_states_recorded: usize = 1;
        let mut num_samples_played: u64 = 0;
        let mut num_states_received: u64 = 0;
        audio_player.play_audio(Box::new(
            move |update_state: bool, listener_pos: Point3<f32>| {
                if update_state {
                    puffin::profile_scope!("update_state");
                    while let Ok(state_vector) = to_audio_consumer.pop() {
                        // Advance the simulation and record history
                        puffin::profile_scope!("advance_simulation");
                        let read_index = index_of_newest;
                        let write_index = (index_of_newest + 1) % SAMPLES_IN_BUFFER;
                        state_history.set_column(write_index, &state_vector);
                        // Compute acceleration
                        acc_history.set_column(
                            write_index,
                            &(player_sample_rate
                                * (state_history
                                    .column(write_index)
                                    .rows(num_points * 3, num_points * 3)
                                    - state_history
                                        .column(read_index)
                                        .rows(num_points * 3, num_points * 3))),
                        );
                        index_of_newest = write_index;
                        num_states_received += 1;
                        num_states_recorded = SAMPLES_IN_BUFFER.min(num_states_recorded + 1);
                        // Send the vector back to avoid deallocating it in this thread.
                        to_ui_producer.push(state_vector);
                    }
                    num_samples_played += 1;
                }
                // Traverse history to find the waves that are contributing to what the listener should be hearing right now.
                puffin::profile_scope!("compute_audio_sample");
                let meters_per_sample = SPEED_OF_SOUND / simulation_sample_rate;
                let mut value = 0.0;
                for point_index in 0..num_points {
                    let point_pos_loc = point_index * 3;

                    // Use latest sample to guess how far back in time we need to go
                    let mut i = {
                        let y = &state_history.column(index_of_newest);
                        let relative_position = Vector3::new(
                            y[point_pos_loc] - listener_pos[0],
                            y[point_pos_loc + 1] - listener_pos[1],
                            y[point_pos_loc + 2] - listener_pos[2],
                        );
                        let distance_by_state = relative_position.norm();
                        let guess_i = (distance_by_state / meters_per_sample).ceil() as usize;
                        guess_i.max(2).min(num_states_recorded - 1)
                    };

                    loop {
                        // Sample from back in time
                        let read_index =
                            (index_of_newest + SAMPLES_IN_BUFFER - i) % SAMPLES_IN_BUFFER;
                        let y = &state_history.column(read_index);
                        let relative_position = Vector3::new(
                            y[point_pos_loc] - listener_pos[0],
                            y[point_pos_loc + 1] - listener_pos[1],
                            y[point_pos_loc + 2] - listener_pos[2],
                        );
                        let distance_by_time = i as f32 * meters_per_sample;
                        let distance_by_time_squared = distance_by_time * distance_by_time;
                        let distance_by_state_squared = relative_position.norm_squared();

                        // Do we need to go further back in time?
                        if distance_by_time_squared < distance_by_state_squared {
                            i += 1;
                            if i >= num_states_recorded {
                                break;
                            }
                            continue;
                        }

                        // Sample from slightly less far back in time
                        let read_index_next = (read_index + 1) % SAMPLES_IN_BUFFER;
                        let y_next = &state_history.column(read_index_next);
                        let relative_position_next = Vector3::new(
                            y_next[point_pos_loc] - listener_pos[0],
                            y_next[point_pos_loc + 1] - listener_pos[1],
                            y_next[point_pos_loc + 2] - listener_pos[2],
                        );
                        let distance_by_time_next = (i - 1) as f32 * meters_per_sample;
                        let distance_by_time_next_squared =
                            distance_by_time_next * distance_by_time_next;
                        let distance_by_state_next_squared = relative_position_next.norm_squared();

                        // Do we need to go forwards in time?
                        if distance_by_time_next_squared > distance_by_state_next_squared {
                            i -= 1;
                            if i < 2 {
                                break;
                            }
                            continue;
                        }

                        // We should now have a sample before and after the information horizon.
                        // Interpolate between these to find the value at the horizon.
                        let distance_by_state = distance_by_state_squared.sqrt();
                        let distance_by_state_prev = distance_by_state_next_squared.sqrt();
                        let t = (distance_by_time - distance_by_state)
                            / (distance_by_state_prev - distance_by_state + meters_per_sample);

                        let acc_next =
                            &acc_history.fixed_view::<3, 1>(point_pos_loc, read_index_next);
                        let acc = &acc_history.fixed_view::<3, 1>(point_pos_loc, read_index);
                        let interpolated_relative_position =
                            relative_position + t * (relative_position_next - relative_position);
                        let interpolated_acc = acc + t * (acc_next - acc);
                        let direction = interpolated_relative_position.normalize();
                        value += interpolated_acc.dot(&direction)
                            / interpolated_relative_position.norm_squared();
                        break;
                    }
                }
                value as f32
            },
        ))?;

        Ok(AudioState {
            listener_pos: todo!(),
            audio_player,
            audio_history: todo!(),
            audio_history_index: todo!(),
            audio_history_resolution: todo!(),
            to_audio_producer,
            to_ui_consumer,
        })
    }
}
