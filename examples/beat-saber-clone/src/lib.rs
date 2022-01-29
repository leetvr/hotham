mod components;
mod resources;
mod systems;

use hotham::{
    components::{
        hand::Handedness,
        panel::{create_panel, PanelButton},
        Pointer,
    },
    gltf_loader::{self, add_model_to_world},
    hecs::{Entity, World},
    resources::{vulkan_context::VulkanContext, RenderContext},
    schedule_functions::{
        begin_frame, begin_pbr_renderpass, end_frame, end_pbr_renderpass, physics_step,
    },
    systems::{
        audio_system, collision_system, draw_gui_system, rendering_system,
        update_parent_transform_matrix_system, update_rigid_body_transforms_system,
        update_transform_matrix_system,
    },
    systems::{pointers_system, Queries},
    Engine, HothamError, HothamResult,
};

use components::Colour;
use resources::GameContext;
use systems::{game::game_system, sabers::add_saber, sabers_system, BeatSaberQueries};

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

    while !engine.should_quit() {
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

    // Game
    game_system(beat_saber_queries, world, game_state, audio_context);

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
    let render_context = &mut engine.render_context;
    let vulkan_context = &engine.vulkan_context;
    let physics_context = &mut engine.physics_context;
    let audio_context = &mut engine.audio_context;
    let gui_context = &engine.gui_context;
    let mut world = World::default();

    let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../assets/beat_saber.glb")];
    let models = gltf_loader::load_models_from_glb(
        &glb_bufs,
        &vulkan_context,
        &render_context.descriptor_set_layouts,
    )?;

    // Add music
    add_music(audio_context);

    // Add environment
    add_environment(&models, &mut world, vulkan_context, render_context);

    // Add sabers
    for colour in [Colour::Blue, Colour::Red] {
        add_saber(
            colour,
            &models,
            &mut world,
            vulkan_context,
            render_context,
            physics_context,
        );
    }

    // Add pointer
    let pointer = add_pointer(&models, &mut world, vulkan_context, render_context);

    // Add panels
    let main_menu_panel_components = create_panel(
        "Main Menu",
        800,
        800,
        vulkan_context,
        render_context,
        gui_context,
        vec![PanelButton::new("Beethoven - Op. 131")],
    );
    let main_menu_panel = world.spawn(main_menu_panel_components);

    // Create game context
    let game_context = GameContext::new(pointer, main_menu_panel);

    Ok((world, game_context))
}

fn add_music(audio_context: &mut hotham::resources::AudioContext) {
    let main_menu_mp3 = include_bytes!("../assets/Cloud Echo - TrackTribe.mp3").to_vec();
    audio_context.add_music_track("Main Menu", main_menu_mp3);
}

fn add_pointer(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> Entity {
    let pointer = add_model_to_world(
        "Blue Pointer",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();

    world
        .insert_one(
            pointer,
            Pointer {
                handedness: Handedness::Right,
                trigger_value: 0.0,
            },
        )
        .unwrap();

    pointer
}

fn add_environment(
    models: &std::collections::HashMap<String, World>,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) {
    add_model_to_world(
        "Environment",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    );
}
