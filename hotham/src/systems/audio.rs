use glam::Vec3;
use hecs::World;
use openxr::SpaceVelocityFlags;

use crate::{
    components::{sound_emitter::SoundState, GlobalTransform, RigidBody, SoundEmitter},
    contexts::{AudioContext, XrContext},
    util::is_space_valid,
    Engine,
};

/// Audio system
/// Walks through each SoundEmitter that has a RigidBody and:
/// - updates its position in space
/// - updates its playing state
pub fn audio_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let audio_context = &mut engine.audio_context;
    let xr_context = &engine.xr_context;

    audio_system_inner(world, audio_context, xr_context);
}

fn audio_system_inner(world: &mut World, audio_context: &mut AudioContext, xr_context: &XrContext) {
    // First, where is the listener?
    let (stage_from_listener, listener_velocity_in_stage) = xr_context
        .view_space
        .relate(
            &xr_context.stage_space,
            xr_context.frame_state.predicted_display_time, // TODO: Use "now" instead.
        )
        .unwrap();

    if !is_space_valid(&stage_from_listener) {
        return;
    }

    if !listener_velocity_in_stage
        .velocity_flags
        .contains(SpaceVelocityFlags::LINEAR_VALID)
    {
        return;
    }

    audio_context.update_listener_rotation(stage_from_listener.pose.orientation.into());

    let listener_position_in_stage: Vec3 =
        mint::Vector3::from(stage_from_listener.pose.position).into();
    let listener_velocity_in_stage: Vec3 =
        mint::Vector3::from(listener_velocity_in_stage.linear_velocity).into();

    for (_, (sound_emitter, rigid_body, global_transform)) in
        world.query_mut::<(&mut SoundEmitter, &RigidBody, &GlobalTransform)>()
    {
        // Get the position and velocity of the entity.
        let (_, _, source_position_in_stage) = global_transform.to_scale_rotation_translation();
        let source_velocity_in_stage = rigid_body.linear_velocity;

        // Compute relative position and velocity
        let relative_position_in_stage =
            (source_position_in_stage - listener_position_in_stage).into();
        let relative_velocity_in_stage =
            (source_velocity_in_stage - listener_velocity_in_stage).into();

        // Determine what we should do with the audio source
        match (sound_emitter.current_state(), &sound_emitter.next_state) {
            (SoundState::Stopped, Some(SoundState::Playing)) => {
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
    use hecs::{Entity, World};
    use std::{
        thread,
        time::{Duration, Instant},
    };

    const DURATION_SECS: u32 = 3;

    use crate::{
        contexts::{audio_context::MusicTrack, PhysicsContext, XrContext},
        HothamError, VIEW_TYPE,
    };

    use super::*;

    #[test]
    pub fn test_audio_system() {
        // Create resources
        let (mut xr_context, _) = XrContext::testing();
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
        let rigid_body = RigidBody {
            linear_velocity: [0.5, 0., 0.].into(),
            ..Default::default()
        };
        let sound_emitter = audio_context.create_sound_emitter(sound_effect);

        // Create world
        let mut world = World::new();
        let audio_entity = world.spawn((sound_emitter, rigid_body));

        // Timers
        let start = Instant::now();

        loop {
            thread::sleep(Duration::from_millis(50));
            let dt = start.elapsed();
            if dt >= Duration::from_secs(DURATION_SECS as u64) {
                break;
            }

            tick(
                &mut xr_context,
                audio_entity,
                &mut world,
                &mut audio_context,
                &start,
                right_here,
                tell_me_that_i_cant,
                &mut physics_context,
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn tick(
        xr_context: &mut XrContext,
        audio_entity: Entity,
        world: &mut World,
        audio_context: &mut AudioContext,
        start: &Instant,
        right_here: MusicTrack,
        tell_me_that_i_cant: MusicTrack,
        physics_context: &mut PhysicsContext,
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
        audio_system_inner(world, audio_context, xr_context);
    }

    fn update_xr(xr_context: &mut XrContext) {
        match xr_context.begin_frame() {
            Err(HothamError::NotRendering) => (),
            Ok(_) => (),
            err => panic!("Error beginning frame: {err:?}"),
        };
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
        let mut source = world.get::<&mut SoundEmitter>(entity).unwrap();
        if let SoundState::Stopped = source.current_state() {
            source.play()
        }

        if start.elapsed().as_secs() >= 2 && audio_context.current_music_track != Some(right_here) {
            audio_context.play_music_track(right_here);
        } else if start.elapsed().as_secs() >= 1
            && start.elapsed().as_secs() < 2
            && audio_context.current_music_track != Some(tell_me_that_i_cant)
        {
            audio_context.play_music_track(tell_me_that_i_cant);
        }
    }
}
