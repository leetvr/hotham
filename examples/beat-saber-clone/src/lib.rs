mod components;
mod resources;
mod systems;

use hotham::{
    components::Visible,
    hecs::World,
    schedule_functions::{
        apply_haptic_feedback, begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass,
        physics_step,
    },
    systems::{
        audio_system, collision_system, draw_gui_system, rendering_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system,
    },
    systems::{pointers_system, Queries},
    xr::{self, SessionState},
    Engine, HothamError, HothamResult,
};

use resources::{
    game_context::{add_songs, add_sound_effects, GameState},
    GameContext,
};
use systems::{game::game_system, sabers_system, BeatSaberQueries};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[BEAT_SABER_EXAMPLE] MAIN!");
    real_main().expect("[BEAT_SABER_EXAMPLE] ERROR IN MAIN!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let (mut world, mut game_context) = init(&mut engine)?;
    let mut hotham_queries = Default::default();
    let mut beat_saber_queries = Default::default();

    while let Ok((previous_state, current_state)) = engine.update() {
        tick(
            previous_state,
            current_state,
            &mut engine,
            &mut world,
            &mut hotham_queries,
            &mut beat_saber_queries,
            &mut game_context,
        );
    }

    Ok(())
}

fn tick(
    previous_state: xr::SessionState,
    current_state: xr::SessionState,
    engine: &mut Engine,
    world: &mut World,
    hotham_queries: &mut Queries,
    beat_saber_queries: &mut BeatSaberQueries,
    game_context: &mut GameContext,
) {
    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;
    let gui_context = &mut engine.gui_context;
    let haptic_context = &mut engine.haptic_context;
    let audio_context = &mut engine.audio_context;

    // If we're not in a session, don't run the frame loop.
    match current_state {
        xr::SessionState::IDLE | xr::SessionState::EXITING | xr::SessionState::STOPPING => return,
        _ => {}
    }

    // Frame start
    begin_frame(xr_context, vulkan_context, render_context);

    handle_state_change(
        previous_state,
        current_state,
        audio_context,
        game_context,
        world,
    );

    // Core logic tasks - these are only necessary when in a FOCUSSED state.
    if current_state == xr::SessionState::FOCUSED {
        // Handle input
        sabers_system(
            &mut beat_saber_queries.sabers_query,
            world,
            xr_context,
            physics_context,
        );
        pointers_system(
            &mut hotham_queries.pointers_query,
            world,
            xr_context,
            physics_context,
        );

        // Physics
        physics_step(physics_context);
        collision_system(&mut hotham_queries.collision_query, world, physics_context);

        // Game logic
        game_system(
            beat_saber_queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );

        // Update the world
        update_rigid_body_transforms_system(
            &mut hotham_queries.update_rigid_body_transforms_query,
            world,
            physics_context,
        );
        update_transform_matrix_system(&mut hotham_queries.update_transform_matrix_query, world);
        update_parent_transform_matrix_system(
            &mut hotham_queries.parent_query,
            &mut hotham_queries.roots_query,
            world,
        );

        // Draw GUI
        draw_gui_system(
            &mut hotham_queries.draw_gui_query,
            world,
            vulkan_context,
            &xr_context.frame_index,
            render_context,
            gui_context,
            haptic_context,
        );

        // Haptics
        apply_haptic_feedback(xr_context, haptic_context);

        // Audio
        audio_system(
            &mut hotham_queries.audio_query,
            world,
            audio_context,
            physics_context,
            xr_context,
        );
    }

    // Rendering tasks - only necessary if we are in at least the visible state
    if current_state == xr::SessionState::VISIBLE || current_state == xr::SessionState::FOCUSED {
        begin_pbr_renderpass(xr_context, vulkan_context, render_context);
        rendering_system(
            &mut hotham_queries.rendering_query,
            world,
            vulkan_context,
            xr_context.frame_index,
            render_context,
        );
        end_pbr_renderpass(xr_context, vulkan_context, render_context);
    }

    // End the frame
    end_frame(xr_context, vulkan_context, render_context);
}

fn handle_state_change(
    previous_state: SessionState,
    current_state: SessionState,
    audio_context: &mut hotham::resources::AudioContext,
    game_context: &mut GameContext,
    world: &mut World,
) {
    let mut objects_to_hide = Vec::new();
    let mut objects_to_show = Vec::new();

    match (previous_state, current_state) {
        (SessionState::VISIBLE, SessionState::FOCUSED) => {
            audio_context.resume_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    objects_to_show.push(game_context.pointer.clone());
                }
                GameState::Playing(_) => {
                    objects_to_show.push(game_context.blue_saber.clone());
                    objects_to_show.push(game_context.red_saber.clone());
                }
            }
        }
        (SessionState::FOCUSED, SessionState::VISIBLE) => {
            audio_context.pause_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    objects_to_hide.push(game_context.pointer.clone());
                }
                GameState::Playing(_) => {
                    objects_to_hide.push(game_context.blue_saber.clone());
                    objects_to_hide.push(game_context.red_saber.clone());
                }
            }
        }
        _ => {}
    }

    for e in objects_to_hide.drain(..) {
        if world.remove_one::<Visible>(e).is_err() {
            println!(
                "[STATE_CHANGE] Tried to make {:?} hidden but it had no Visible component",
                e
            )
        }
    }

    for e in objects_to_show.drain(..) {
        world.insert_one(e, Visible {}).unwrap();
    }
}

fn init(engine: &mut Engine) -> Result<(World, GameContext), HothamError> {
    let mut world = World::default();
    let mut game_context = GameContext::new(engine, &mut world);
    add_songs(&mut engine.audio_context, &mut game_context);
    add_sound_effects(&mut engine.audio_context, &mut game_context);
    Ok((world, game_context))
}
