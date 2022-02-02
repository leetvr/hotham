mod components;
mod resources;
mod systems;

use hotham::{
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
    xr, Engine, HothamError, HothamResult,
};

use resources::{
    game_context::{add_songs, add_sound_effects},
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

    while engine.update()? {
        tick(
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
    engine: &mut Engine,
    world: &mut World,
    hotham_queries: &mut Queries,
    beat_saber_queries: &mut BeatSaberQueries,
    game_state: &mut GameContext,
) {
    let xr_context = &mut engine.xr_context;
    let vulkan_context = &engine.vulkan_context;
    let render_context = &mut engine.render_context;
    let physics_context = &mut engine.physics_context;
    let gui_context = &mut engine.gui_context;
    let haptic_context = &mut engine.haptic_context;
    let audio_context = &mut engine.audio_context;

    // If we're not in a session, don't run the frame loop.
    match xr_context.session_state {
        xr::SessionState::IDLE | xr::SessionState::EXITING | xr::SessionState::STOPPING => return,
        _ => {}
    }

    // Frame start
    begin_frame(xr_context, vulkan_context, render_context);

    // Input
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

    // Game
    game_system(
        beat_saber_queries,
        world,
        game_state,
        audio_context,
        physics_context,
        haptic_context,
    );

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

    // GUI
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

    // Render
    begin_pbr_renderpass(xr_context, vulkan_context, render_context);
    rendering_system(
        &mut hotham_queries.rendering_query,
        world,
        vulkan_context,
        xr_context.frame_index,
        render_context,
    );
    end_pbr_renderpass(xr_context, vulkan_context, render_context);
    end_frame(xr_context, vulkan_context, render_context);
}

fn init(engine: &mut Engine) -> Result<(World, GameContext), HothamError> {
    let mut world = World::default();
    let mut game_context = GameContext::new(engine, &mut world);
    add_songs(&mut engine.audio_context, &mut game_context);
    add_sound_effects(&mut engine.audio_context, &mut game_context);
    Ok((world, game_context))
}
