use legion::system;

use crate::{components::AudioSource, resources::AudioContext};

#[system(for_each)]
pub fn audio(audio_source: &mut AudioSource, #[resource] audio_context: &mut AudioContext) {}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use legion::{Resources, Schedule, World};
    const DURATION_SECS: u32 = 6;

    use super::*;
    #[test]
    pub fn test_audio_system() {
        // Test that we can play an MP3 from disk.
        let test_mp3 = include_bytes!("../../../test_assets/gymnopedie.mp3");
        let mut audio_context = AudioContext::default();
        let audio_source = audio_context.create_audio_source(test_mp3);

        let mut world = World::default();
        let mut resources = Resources::default();

        world.push((audio_source,));

        let mut schedule = Schedule::builder().add_system(audio_system()).build();

        let start = Instant::now();

        loop {
            thread::sleep(Duration::from_millis(50));
            let dt = start.elapsed();
            if dt >= Duration::from_secs(DURATION_SECS as u64) {
                break;
            }

            schedule.execute(&mut world, &mut resources);

            // // Access our Spatial Controls
            // let mut spatial_control = signal.control::<oddio::SpatialBuffered<_>, _>();

            // // This has no noticable effect because it matches the initial velocity, but serves to
            // // demonstrate that `Spatial` can smooth over the inevitable small timing inconsistencies
            // // between the main thread and the audio thread without glitching.
            // spatial_control.set_motion(
            //     [-SPEED + SPEED * dt.as_secs_f32(), 10.0, 0.0].into(),
            //     [SPEED, 0.0, 0.0].into(),
            //     false,
            // );

            // // We also could adjust the Gain here in the same way:
            // let mut gain_control = signal.control::<oddio::Gain<_>, _>();

            // // Just leave the gain at its natural volume. (sorry this can be a bit loud!)
            // gain_control.set_gain(1.0);
        }
    }
}
