use std::time::{Duration, Instant};

use hotham::anyhow;
use nalgebra::{self, DMatrix, DVector, Point3, Vector3};

use crate::audio_player::AudioPlayer;

pub struct AudioSimulationUpdate {
    pub state_vector: DVector<f32>,
    pub simulation_time: Instant,
}

pub struct AudioState {
    pub audio_player: AudioPlayer,
    pub to_audio_producer: rtrb::Producer<AudioSimulationUpdate>,
    pub to_ui_consumer: rtrb::Consumer<AudioSimulationUpdate>,
}

impl AudioState {
    pub fn init_audio(num_points: usize, simulation_time: Instant) -> anyhow::Result<AudioState> {
        puffin::profile_function!();
        let (to_audio_producer, mut to_audio_consumer) =
            rtrb::RingBuffer::<AudioSimulationUpdate>::new(100);
        let (mut to_ui_producer, to_ui_consumer) =
            rtrb::RingBuffer::<AudioSimulationUpdate>::new(100);

        let mut audio_player = AudioPlayer::new()?;
        let player_sample_rate = audio_player.config.sample_rate().0 as f32;

        // Keep a history of states in a circular buffer so that we can create a waveform by combining contributions over time.
        const SAMPLES_IN_BUFFER: usize = 1024;
        const SPEED_OF_SOUND: f32 = 343.0;
        const SIMULATION_RATE: f32 = 1000.0;
        let simulation_timestep = Duration::from_secs_f32(1.0 / SIMULATION_RATE);
        let normal_audio_delay = Duration::from_millis(15);

        let mut state_history = DMatrix::<f32>::zeros(num_points * 3 * 2, SAMPLES_IN_BUFFER);
        let mut acc_history = DMatrix::<f32>::zeros(num_points * 3, SAMPLES_IN_BUFFER);
        let mut index_of_newest: usize = 0;
        let mut num_states_recorded: usize = 1;
        let mut latest_audio_sample_timestamp = simulation_time;
        let mut latest_simulation_timestamp = simulation_time;
        let normal_time_per_audio_sample = Duration::from_secs_f32(1.0 / player_sample_rate);
        audio_player.play_audio(Box::new(
            move |update_state: bool, listener_pos: &Point3<f32>| {
                if update_state {
                    puffin::profile_scope!("update_state");
                    while let Ok(message) = to_audio_consumer.pop() {
                        // Advance the simulation and record history
                        puffin::profile_scope!("advance_simulation");
                        let read_index = index_of_newest;
                        let write_index = (index_of_newest + 1) % SAMPLES_IN_BUFFER;
                        state_history.set_column(write_index, &message.state_vector);
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
                        num_states_recorded = SAMPLES_IN_BUFFER.min(num_states_recorded + 1);
                        latest_simulation_timestamp += simulation_timestep;
                        // Send the message back to avoid deallocating it in this thread.
                        let _ = to_ui_producer.push(message);
                    }
                    if latest_audio_sample_timestamp + normal_audio_delay
                        < latest_simulation_timestamp
                    {
                        latest_audio_sample_timestamp += normal_time_per_audio_sample;
                    }
                    // Skip ahead if we are too far back
                    if latest_audio_sample_timestamp + 2 * normal_audio_delay
                        < latest_simulation_timestamp
                    {
                        latest_audio_sample_timestamp =
                            latest_simulation_timestamp - normal_audio_delay;
                    }
                }
                // Traverse history to find the waves that are contributing to what the listener should be hearing right now.
                puffin::profile_scope!("compute_audio_sample");
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
                        let emission_time_by_state = latest_audio_sample_timestamp
                            - Duration::from_secs_f32(distance_by_state / SPEED_OF_SOUND);
                        let float_i = (latest_simulation_timestamp - emission_time_by_state)
                            .as_secs_f32()
                            / simulation_timestep.as_secs_f32();
                        let guess_i = (float_i).ceil() as usize;
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
                        let distance_to_i = relative_position.norm();
                        let time_of_emission_of_i =
                            latest_simulation_timestamp - simulation_timestep * i as u32;
                        let time_of_arrival_of_i = time_of_emission_of_i
                            + Duration::from_secs_f32(distance_to_i / SPEED_OF_SOUND);

                        // Do we need to go further back in time?
                        if time_of_arrival_of_i > latest_audio_sample_timestamp {
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
                        let distance_to_next = relative_position_next.norm();
                        let time_of_emission_of_next = time_of_emission_of_i + simulation_timestep;
                        let time_of_arrival_of_next = time_of_emission_of_next
                            + Duration::from_secs_f32(distance_to_next / SPEED_OF_SOUND);

                        // Do we need to go forwards in time?
                        if latest_audio_sample_timestamp > time_of_arrival_of_next {
                            i -= 1;
                            if i < 2 {
                                break;
                            }
                            continue;
                        }

                        // We should now have a sample before and after the information horizon.
                        // time_of_arrival_of_i ≤ latest_audio_sample_timestamp ≤ time_of_arrival_of_next
                        // Interpolate between these to find the value at the horizon.
                        let t = (latest_audio_sample_timestamp - time_of_arrival_of_i)
                            .as_secs_f32()
                            / simulation_timestep.as_secs_f32();
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
                value
            },
        ))?;

        Ok(AudioState {
            audio_player,
            to_audio_producer,
            to_ui_consumer,
        })
    }
}
