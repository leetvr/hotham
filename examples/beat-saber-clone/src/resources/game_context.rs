use hotham::{
    components::panel::{create_panel, PanelButton},
    gltf_loader,
    hecs::{Entity, World},
    resources::audio_context::MusicTrack,
    Engine,
};

use crate::{add_environment, add_pointer, components::Colour, systems::sabers::add_saber};

#[derive(Debug, Clone, PartialEq)]
pub struct GameContext {
    pub current_score: usize,
    pub state: GameState,
    pub pointer: Entity,
    pub main_menu_panel: Entity,
    pub blue_saber: Entity,
    pub red_saber: Entity,
}

impl GameContext {
    pub fn new(engine: &mut Engine, world: &mut World) -> Self {
        let render_context = &mut engine.render_context;
        let vulkan_context = &engine.vulkan_context;
        let physics_context = &mut engine.physics_context;
        let audio_context = &mut engine.audio_context;
        let gui_context = &engine.gui_context;

        let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../../assets/beat_saber.glb")];
        let models = gltf_loader::load_models_from_glb(
            &glb_bufs,
            &vulkan_context,
            &render_context.descriptor_set_layouts,
        )
        .expect("Unable to load models!");

        // Add music
        add_music(audio_context);

        // Add environment
        add_environment(&models, world, vulkan_context, render_context);

        // Add sabers
        let sabers = [Colour::Blue, Colour::Red].map(|colour| {
            add_saber(
                colour,
                &models,
                world,
                vulkan_context,
                render_context,
                physics_context,
            )
        });

        // Add pointer
        let pointer = add_pointer(&models, world, vulkan_context, render_context);

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
        Self {
            pointer,
            main_menu_panel,
            current_score: 0,
            state: GameState::Init,
            blue_saber: sabers[0],
            red_saber: sabers[1],
        }
    }
}

fn add_music(audio_context: &mut hotham::resources::AudioContext) {
    let main_menu_mp3 = include_bytes!("../../assets/Cloud Echo - TrackTribe.mp3").to_vec();
    audio_context.add_music_track("Main Menu", main_menu_mp3);

    let beethoven = include_bytes!("../../assets/Quartet 14 - Beethoven.mp3").to_vec();
    audio_context.add_music_track("Beethoven", beethoven);
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
    Playing(MusicTrack),
}
