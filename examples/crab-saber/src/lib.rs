mod components;
mod resources;
mod systems;

use hotham::{
    components::Visible,
    hecs::{Entity, World},
    schedule_functions::{apply_haptic_feedback, physics_step},
    systems::{
        audio_system, collision_system, draw_gui_system, rendering_system,
        update_global_transform_system, update_global_transform_with_parent_system,
        update_local_transform_with_rigid_body_system,
    },
    systems::{pointers_system, Queries},
    xr::{self, SessionState},
    Engine, HothamResult, TickData,
};

use resources::{
    game_context::{add_songs, add_sound_effects, GameState},
    GameContext,
};
use systems::{game::game_system, sabers_system, CrabSaberQueries};

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[CRAB_SABER] MAIN!");
    real_main().expect("[CRAB_SABER] ERROR IN MAIN!");
}

pub fn real_main() -> HothamResult<()> {
    let mut engine = Engine::new();
    let (mut world, mut game_context) = init(&mut engine);
    let mut hotham_queries = Default::default();
    let mut crab_saber_queries = Default::default();

    while let Ok(tick_data) = engine.update() {
        tick(
            tick_data,
            &mut engine,
            &mut world,
            &mut hotham_queries,
            &mut crab_saber_queries,
            &mut game_context,
        );
        engine.finish()?;
    }

    Ok(())
}

fn tick(
    tick_data: TickData,
    engine: &mut Engine,
    world: &mut World,
    hotham_queries: &mut Queries,
    crab_saber_queries: &mut CrabSaberQueries,
    game_context: &mut GameContext,
) {
    let xr_context = &mut engine.xr_context;
    let input_context = &mut engine.input_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;
    let gui_context = &mut engine.gui_context;
    let haptic_context = &mut engine.haptic_context;
    let audio_context = &mut engine.audio_context;

    handle_state_change(&tick_data, audio_context, game_context, world);

    // Simulation tasks - these are only necessary in the focussed state.
    if tick_data.current_state == xr::SessionState::FOCUSED {
        // Handle input
        sabers_system(
            &mut crab_saber_queries.sabers_query,
            world,
            input_context,
            physics_context,
        );
        pointers_system(
            &mut hotham_queries.pointers_query,
            world,
            input_context,
            physics_context,
        );

        // Physics
        physics_step(physics_context);
        collision_system(&mut hotham_queries.collision_query, world, physics_context);

        // Game logic
        game_system(
            crab_saber_queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );

        // Update the world
        update_local_transform_with_rigid_body_system(
            &mut hotham_queries.update_rigid_body_transforms_query,
            world,
            physics_context,
        );
        update_global_transform_system(&mut hotham_queries.update_global_transform_query, world);
        update_global_transform_with_parent_system(
            &mut hotham_queries.parent_query,
            &mut hotham_queries.roots_query,
            world,
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

    // Draw GUI
    draw_gui_system(
        &mut hotham_queries.draw_gui_query,
        world,
        vulkan_context,
        render_context,
        gui_context,
        haptic_context,
    );

    // Update the views - we always want to do this at the last possible minute to get an accurate position of the player's head.
    let views = xr_context.update_views();

    // Draw objects
    rendering_system(
        &mut hotham_queries.rendering_query,
        world,
        vulkan_context,
        render_context,
        views,
        tick_data.swapchain_image_index,
    );
}

fn handle_state_change(
    tick_data: &TickData,
    audio_context: &mut hotham::resources::AudioContext,
    game_context: &mut GameContext,
    world: &mut World,
) {
    match (tick_data.previous_state, tick_data.current_state) {
        (SessionState::VISIBLE, SessionState::FOCUSED) => {
            audio_context.resume_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    show(world, game_context.pointer);
                }
                GameState::Playing(_) => {
                    show(world, game_context.blue_saber);
                    show(world, game_context.red_saber);
                }
            }
        }
        (SessionState::FOCUSED, SessionState::VISIBLE) => {
            audio_context.pause_music_track();
            match game_context.state {
                GameState::Init => {}
                GameState::MainMenu | GameState::GameOver => {
                    hide(world, game_context.pointer);
                }
                GameState::Playing(_) => {
                    hide(world, game_context.blue_saber);
                    hide(world, game_context.red_saber);
                }
            }
        }
        _ => {}
    }
}

fn init(engine: &mut Engine) -> (World, GameContext) {
    let mut world = World::default();
    let mut game_context = GameContext::new(engine, &mut world);
    add_songs(&mut engine.audio_context, &mut game_context);
    add_sound_effects(&mut engine.audio_context, &mut game_context);
    (world, game_context)
}

fn hide(world: &mut World, entity: Entity) {
    if world.remove_one::<Visible>(entity).is_err() {
        println!(
            "[STATE_CHANGE] Tried to make {:?} hidden but it had no Visible component",
            entity
        )
    }
}

fn show(world: &mut World, entity: Entity) {
    world.insert_one(entity, Visible {}).unwrap();
}
