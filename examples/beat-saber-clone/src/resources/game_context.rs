use std::{collections::HashMap, fmt::Debug};

use hotham::{
    components::{
        hand::Handedness,
        panel::{create_panel, PanelButton},
        Pointer,
    },
    gltf_loader::{self, add_model_to_world},
    hecs::{Entity, World},
    resources::{audio_context::MusicTrack, vulkan_context::VulkanContext, RenderContext},
    Engine,
};

use crate::{components::Colour, systems::sabers::add_saber};

pub struct GameContext {
    pub current_score: usize,
    pub state: GameState,
    pub pointer: Entity,
    pub main_menu_panel: Entity,
    pub blue_saber: Entity,
    pub red_saber: Entity,
    pub songs: HashMap<String, Song>,
    pub models: HashMap<String, World>,
}

impl Debug for GameContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GameContext")
            .field("current_score", &self.current_score)
            .field("state", &self.state)
            .field("music_tracks", &self.songs)
            .finish()
    }
}

impl GameContext {
    pub fn new(engine: &mut Engine, world: &mut World) -> Self {
        let render_context = &mut engine.render_context;
        let vulkan_context = &engine.vulkan_context;
        let physics_context = &mut engine.physics_context;
        let gui_context = &engine.gui_context;

        let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../../assets/beat_saber.glb")];
        let models = gltf_loader::load_models_from_glb(
            &glb_bufs,
            &vulkan_context,
            &render_context.descriptor_set_layouts,
        )
        .expect("Unable to load models!");

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
            vec![PanelButton::new("Spence - Right Here Beside You")],
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
            songs: Default::default(),
            models,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub track: MusicTrack,
    pub beat_length: f32,
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

pub fn add_music(
    audio_context: &mut hotham::resources::AudioContext,
    game_context: &mut GameContext,
) {
    let main_menu_mp3 = include_bytes!("../../assets/TrackTribe - Cloud Echo.mp3").to_vec();
    game_context.songs.insert(
        "Main Menu".to_string(),
        Song {
            beat_length: 0.,
            track: audio_context.add_music_track(main_menu_mp3),
        },
    );

    let right_here_beside_you =
        include_bytes!("../../assets/Spence - Right Here Beside You.mp3").to_vec();
    game_context.songs.insert(
        "Spence - Right Here Beside You".to_string(),
        Song {
            beat_length: 129. / 60.,
            track: audio_context.add_music_track(right_here_beside_you),
        },
    );
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
    Playing(Song),
}
