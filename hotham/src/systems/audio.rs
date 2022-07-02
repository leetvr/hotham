use hecs::{PreparedQuery, World};
use nalgebra::{Point3, Vector3};
use openxr::{SpaceLocationFlags, SpaceVelocityFlags};

use crate::{
    components::{sound_emitter::SoundState, RigidBody, SoundEmitter},
    resources::{AudioContext, PhysicsContext, XrContext},
};

/// Audio system
/// Walks through each SoundEmitter that has a RigidBody and:
/// - updates its position in space
/// - updates its playing state
pub fn audio_system(
    query: &mut PreparedQuery<(&mut SoundEmitter, &RigidBody)>,
    world: &mut World,
    audio_context: &mut AudioContext,
    physics_context: &PhysicsContext,
    xr_context: &XrContext,
) {
    // First, where is the listener?
    let (listener_position_in_stage, listener_velocity_in_stage) = xr_context
        .view_space
        .relate(
            &xr_context.stage_space,
            xr_context.frame_state.predicted_display_time, // TODO: Use "now" instead.
        )
        .unwrap();

    if !listener_position_in_stage
        .location_flags
        .contains(SpaceLocationFlags::POSITION_VALID)
    {
        return;
    }

    if !listener_velocity_in_stage
        .velocity_flags
        .contains(SpaceVelocityFlags::LINEAR_VALID)
    {
        return;
    }

    let listener_position_in_stage: Vector3<_> =
        mint::Vector3::from(listener_position_in_stage.pose.position).into();
    let listener_velocity_in_stage: Vector3<_> =
        mint::Vector3::from(listener_velocity_in_stage.linear_velocity).into();

    for (_, (sound_emitter, rigid_body)) in query.query_mut(world) {
        // Get the position and velocity of the entity.
        let rigid_body = physics_context
            .rigid_bodies
            .get(rigid_body.handle)
            .expect("Unable to get RigidBody");

        let source_position_in_stage: Point3<_> = (*rigid_body.translation()).into();
        let source_velocity_in_stage: Vector3<_> = *rigid_body.linvel();

        // Compute relative position and velocity
        let relative_position_in_stage =
            (source_position_in_stage - listener_position_in_stage).into();
        let relative_velocity_in_stage =
            (source_velocity_in_stage - listener_velocity_in_stage).into();

        // Determine what we should do with the audio source
        match (sound_emitter.current_state(), &sound_emitter.next_state) {
            (SoundState::Stopped, Some(SoundState::Playing)) => {
                println!(
                    "[HOTHAM_AUDIO] - Playing sound effect at {:?}, {:?} from {:?}!. Original position: {:?}",
                    relative_position_in_stage, source_velocity_in_stage, listener_position_in_stage, rigid_body.translation()
                );
                audio_context.play_audio(
                    sound_emitter,
                    relative_position_in_stage,
                    relative_velocity_in_stage,
                );
            }
            (SoundState::Paused, Some(SoundState::Playing)) => {
                audio_context.resume_audio(sound_emitter);
            }
            (SoundState::Playing | SoundState::Paused, Some(SoundState::Paused)) => {
                audio_context.pause_audio(sound_emitter);
            }
            (SoundState::Stopped, Some(SoundState::Stopped)) => {
                // Do nothing
            }
            (_, Some(SoundState::Stopped)) => {
                audio_context.stop_audio(sound_emitter);
            }
            _ => {}
        }

        // Reset the sound emitter's intent
        sound_emitter.next_state = None;

        // Update its position and velocity
        audio_context.update_motion(
            sound_emitter,
            relative_position_in_stage,
            relative_velocity_in_stage,
        );
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use hecs::{Entity, PreparedQuery, World};
    use std::{
        thread,
        time::{Duration, Instant},
    };

    use rapier3d::prelude::RigidBodyBuilder;
    const DURATION_SECS: u32 = 8;

    use crate::{
        resources::{audio_context::MusicTrack, XrContext},
        VIEW_TYPE,
    };

    use super::*;

    #[test]
    pub fn test_audio_system() {
        // Create resources
        let (mut xr_context, _) = XrContext::new().unwrap();
        let mut audio_context = AudioContext::default();
        let mut physics_context = PhysicsContext::default();

        // Load MP3s from disk
        let right_here =
            include_bytes!("../../../test_assets/right_here_beside_you_clip.mp3").to_vec();
        let tell_me_that_i_cant =
            include_bytes!("../../../test_assets/tell_me_that_i_cant_clip.mp3").to_vec();

        let right_here = audio_context.add_music_track(right_here);
        let tell_me_that_i_cant = audio_context.add_music_track(tell_me_that_i_cant);

        // Create rigid body for the test entity
        let sound_effect = include_bytes!("../../../test_assets/ice_crash.mp3").to_vec();
        let rigid_body = RigidBodyBuilder::new_dynamic()
            .linvel([0.5, 0., 0.].into())
            .translation([-2., 0., 0.].into())
            .build();
        let handle = physics_context.rigid_bodies.insert(rigid_body);
        let rigid_body = RigidBody { handle };
        let sound_emitter = audio_context.create_sound_emitter(sound_effect);

        // Create world
        let mut world = World::new();
        let audio_entity = world.spawn((sound_emitter, rigid_body));

        // Create query
        let mut audio_query = PreparedQuery::<(&mut SoundEmitter, &RigidBody)>::default();

        // Timers
        let start = Instant::now();

        loop {
            thread::sleep(Duration::from_millis(50));
            let dt = start.elapsed();
            if dt >= Duration::from_secs(DURATION_SECS as u64) {
                break;
            }

            schedule(
                &mut xr_context,
                audio_entity,
                &mut world,
                &mut audio_context,
                &start,
                right_here,
                tell_me_that_i_cant,
                &mut physics_context,
                &mut audio_query,
            );
        }
    }

    fn schedule(
        xr_context: &mut XrContext,
        audio_entity: Entity,
        world: &mut World,
        audio_context: &mut AudioContext,
        start: &Instant,
        right_here: MusicTrack,
        tell_me_that_i_cant: MusicTrack,
        physics_context: &mut PhysicsContext,
        audio_query: &mut PreparedQuery<(&mut SoundEmitter, &RigidBody)>,
    ) {
        update_xr(xr_context);
        update_audio(
            audio_entity,
            world,
            audio_context,
            start,
            right_here,
            tell_me_that_i_cant,
        );
        physics_context.update();
        xr_context.end_frame().unwrap();
        audio_system(
            audio_query,
            world,
            audio_context,
            physics_context,
            xr_context,
        );
    }

    fn update_xr(xr_context: &mut XrContext) {
        xr_context.begin_frame().unwrap();
        let (view_state_flags, views) = xr_context
            .session
            .locate_views(
                VIEW_TYPE,
                xr_context.frame_state.predicted_display_time,
                &xr_context.stage_space,
            )
            .unwrap();
        xr_context.views = views;
        xr_context.view_state_flags = view_state_flags;
    }

    fn update_audio(
        entity: Entity,
        world: &mut World,
        audio_context: &mut AudioContext,
        start: &Instant,
        right_here: MusicTrack,
        tell_me_that_i_cant: MusicTrack,
    ) {
        let mut source = world.get_mut::<SoundEmitter>(entity).unwrap();
        match source.current_state() {
            SoundState::Stopped => source.play(),
            _ => {}
        }

        if start.elapsed().as_secs() >= 4 && audio_context.current_music_track != Some(right_here) {
            audio_context.play_music_track(right_here);
        } else if start.elapsed().as_secs() >= 2
            && start.elapsed().as_secs() < 4
            && audio_context.current_music_track != Some(tell_me_that_i_cant)
        {
            audio_context.play_music_track(tell_me_that_i_cant);
        }
    }
}
