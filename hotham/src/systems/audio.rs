use legion::system;
use nalgebra::Point3;

use crate::{
    components::{sound_emitter::SoundState, RigidBody, SoundEmitter},
    resources::{AudioContext, PhysicsContext, XrContext},
    util::posef_to_isometry,
};

#[system(for_each)]
pub fn audio(
    sound_emitter: &mut SoundEmitter,
    #[resource] audio_context: &mut AudioContext,
    #[resource] physics_context: &PhysicsContext,
    #[resource] xr_context: &XrContext,
    rigid_body: &RigidBody,
) {
    // First, where is the listener?
    let listener_location = posef_to_isometry(xr_context.views[1].pose)
        .lerp_slerp(&posef_to_isometry(xr_context.views[0].pose), 0.5);

    // Get the position and velocity of the entity.
    let rigid_body = physics_context
        .rigid_bodies
        .get(rigid_body.handle)
        .expect("Unable to get RigidBody");

    let velocity = (*rigid_body.linvel()).into();

    // Now transform the position of the entity w.r.t. the listener
    let position = listener_location
        .transform_point(&Point3::from(*rigid_body.translation()))
        .into();

    // Determine what we should do with the audio source
    match (sound_emitter.current_state(), &sound_emitter.next_state) {
        (SoundState::Stopped, SoundState::Playing) => {
            audio_context.play_audio(sound_emitter, position, velocity);
        }
        (SoundState::Paused, SoundState::Playing) => {
            audio_context.resume_audio(sound_emitter);
        }
        (SoundState::Playing | SoundState::Paused, SoundState::Paused) => {
            audio_context.pause_audio(sound_emitter);
        }
        (_, SoundState::Stopped) => {
            audio_context.stop_audio(sound_emitter);
        }
        _ => {}
    }

    // Update its position and velocity
    audio_context.update_motion(sound_emitter, position, velocity);
}

#[cfg(test)]
mod tests {
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use legion::{IntoQuery, Resources, Schedule, World};
    use rapier3d::prelude::RigidBodyBuilder;
    const DURATION_SECS: u32 = 8;

    use crate::{resources::XrContext, VIEW_TYPE};

    use super::*;
    #[test]
    pub fn test_audio_system() {
        // Create resources
        let (xr_context, _) = XrContext::new().unwrap();
        let mut audio_context = AudioContext::default();
        let mut physics_context = PhysicsContext::default();

        // Load MP3s from disk
        let beethoven = include_bytes!("../../../test_assets/Quartet 14 - Clip.mp3").to_vec();
        let right_here =
            include_bytes!("../../../test_assets/right_here_beside_you_clip.mp3").to_vec();
        let tell_me_that_i_cant =
            include_bytes!("../../../test_assets/tell_me_that_i_cant_clip.mp3").to_vec();

        let beethoven = audio_context.add_music_track(beethoven);
        let right_here = audio_context.add_music_track(right_here);
        let tell_me_that_i_cant = audio_context.add_music_track(tell_me_that_i_cant);
        audio_context.play_music_track(beethoven);

        // Create rigid body for the test entity
        let sound_effect = include_bytes!("../../../test_assets/ice_crash.mp3").to_vec();
        let rigid_body = RigidBodyBuilder::new_dynamic()
            .linvel([0.5, 0., 0.].into())
            .translation([-2., 0., 0.].into())
            .build();
        let handle = physics_context.rigid_bodies.insert(rigid_body);
        let rigid_body = RigidBody { handle };
        let sound_emitter = audio_context.create_audio_source(sound_effect);

        // Create world
        let mut world = World::default();
        let audio_entity = world.push((sound_emitter, rigid_body));

        // Create resources
        let mut resources = Resources::default();
        resources.insert(audio_context);
        resources.insert(physics_context);
        resources.insert(xr_context);
        let start = Instant::now();

        let mut schedule = Schedule::builder()
            .add_thread_local_fn(move |world, resources| {
                let mut query = <(&mut SoundEmitter, &mut RigidBody)>::query();
                let mut xr_context = resources.get_mut::<XrContext>().unwrap();
                let mut physics_context = resources.get_mut::<PhysicsContext>().unwrap();
                let mut audio_context = resources.get_mut::<AudioContext>().unwrap();

                let (source, _) = query.get_mut(world, audio_entity).unwrap();

                let (frame_state, _) = xr_context.begin_frame().unwrap();
                let (view_state_flags, views) = xr_context
                    .session
                    .locate_views(
                        VIEW_TYPE,
                        frame_state.predicted_display_time,
                        &xr_context.reference_space,
                    )
                    .unwrap();
                xr_context.views = views;
                xr_context.view_state_flags = view_state_flags;

                match source.current_state() {
                    SoundState::Stopped => source.play(),
                    _ => {}
                }

                if start.elapsed().as_secs() >= 4
                    && audio_context.current_music_track.unwrap() != right_here
                {
                    audio_context.play_music_track(right_here);
                } else if start.elapsed().as_secs() >= 2
                    && start.elapsed().as_secs() < 4
                    && audio_context.current_music_track.unwrap() != tell_me_that_i_cant
                {
                    audio_context.play_music_track(tell_me_that_i_cant);
                }

                physics_context.update();
                xr_context.end_frame().unwrap();
            })
            .add_system(audio_system())
            .build();

        loop {
            thread::sleep(Duration::from_millis(50));
            let dt = start.elapsed();
            if dt >= Duration::from_secs(DURATION_SECS as u64) {
                break;
            }

            schedule.execute(&mut world, &mut resources);
        }
    }
}
