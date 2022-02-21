use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

use hotham::{
    components::{
        hand::Handedness, panel::add_panel_to_world, Collider, Pointer, RigidBody, SoundEmitter,
        Visible,
    },
    gltf_loader::{self, add_model_to_world},
    hecs::{Entity, World},
    rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, InteractionGroups, RigidBodyBuilder,
    },
    resources::{
        audio_context::MusicTrack, physics_context::DEFAULT_COLLISION_GROUP,
        vulkan_context::VulkanContext, AudioContext, PhysicsContext, RenderContext,
    },
    Engine,
};
use rand::prelude::*;

use crate::{
    components::{Colour, Cube},
    systems::sabers::add_saber,
};

pub struct GameContext {
    pub current_score: i32,
    pub state: GameState,
    pub pointer: Entity,
    pub main_menu_panel: Entity,
    pub score_panel: Entity,
    pub blue_saber: Entity,
    pub red_saber: Entity,
    pub backstop: Entity,
    pub songs: HashMap<String, Song>,
    pub models: HashMap<String, World>,
    pub last_spawn_time: Instant,
    pub sound_effects: HashMap<String, SoundEmitter>,
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

        let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../../assets/crab_saber.glb")];
        let models = gltf_loader::load_models_from_glb(
            &glb_bufs,
            vulkan_context,
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

        // Spawn cubes
        for _ in 0..20 {
            pre_spawn_cube(
                world,
                &models,
                vulkan_context,
                render_context,
                physics_context,
            );
        }

        // Add pointer
        let pointer = add_pointer(&models, world, vulkan_context, render_context);

        // Add backstop
        let backstop = add_backstop(world, physics_context);

        // Add panels
        let main_menu_panel = add_panel_to_world(
            "Main Menu",
            800,
            800,
            vec![],
            [0., 1., -1.].into(),
            vulkan_context,
            render_context,
            gui_context,
            physics_context,
            world,
        );

        // Add panels
        let score_panel = add_panel_to_world(
            "Current Score: 0",
            300,
            600,
            vec![],
            [-1.25, 1., -2.].into(),
            vulkan_context,
            render_context,
            gui_context,
            physics_context,
            world,
        );

        // Create game context
        Self {
            pointer,
            backstop,
            main_menu_panel,
            score_panel,
            current_score: 0,
            state: GameState::Init,
            blue_saber: sabers[0],
            red_saber: sabers[1],
            songs: Default::default(),
            models,
            last_spawn_time: Instant::now() - Duration::new(100, 0),
            sound_effects: Default::default(),
        }
    }
}

fn add_backstop(
    world: &mut World,
    physics_context: &mut hotham::resources::PhysicsContext,
) -> Entity {
    let collider = ColliderBuilder::cuboid(1., 1., 0.1)
        .translation([0., 1., 1.].into())
        .sensor(true)
        .collision_groups(InteractionGroups::new(
            DEFAULT_COLLISION_GROUP,
            DEFAULT_COLLISION_GROUP,
        ))
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build();

    let handle = physics_context.colliders.insert(collider);
    world.spawn((Collider::new(handle),))
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

    add_model_to_world(
        "Ramp",
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    );
}

pub fn add_songs(audio_context: &mut AudioContext, game_context: &mut GameContext) {
    let main_menu_mp3 = include_bytes!("../../assets/TrackTribe - Cloud Echo.mp3").to_vec();
    game_context.songs.insert(
        "Main Menu".to_string(),
        Song {
            beat_length: Duration::new(0, 0),
            track: audio_context.add_music_track(main_menu_mp3),
        },
    );

    let game_over_mp3 = include_bytes!("../../assets/Chasms - Dark Matter.mp3").to_vec();
    game_context.songs.insert(
        "Game Over".to_string(),
        Song {
            beat_length: Duration::new(0, 0),
            track: audio_context.add_music_track(game_over_mp3),
        },
    );

    let right_here_beside_you =
        include_bytes!("../../assets/Spence - Right Here Beside You.mp3").to_vec();
    game_context.songs.insert(
        "Spence - Right Here Beside You".to_string(),
        Song {
            beat_length: Duration::from_millis(60_000 / 129),
            track: audio_context.add_music_track(right_here_beside_you),
        },
    );

    let tell_me_that_i_cant =
        include_bytes!("../../assets/NEFFEX - Tell Me That I Can't.mp3").to_vec();
    game_context.songs.insert(
        "NEFFEX - Tell Me That I Can't".to_string(),
        Song {
            beat_length: Duration::from_millis(60_000 / 70),
            track: audio_context.add_music_track(tell_me_that_i_cant),
        },
    );
}

pub fn add_sound_effects(audio_context: &mut AudioContext, game_context: &mut GameContext) {
    let hit_mp3 = include_bytes!("../../assets/Hit.mp3").to_vec();
    game_context.sound_effects.insert(
        "Hit".to_string(),
        audio_context.create_sound_emitter(hit_mp3),
    );

    let miss_mp3 = include_bytes!("../../assets/Miss.mp3").to_vec();
    game_context.sound_effects.insert(
        "Miss".to_string(),
        audio_context.create_sound_emitter(miss_mp3),
    );
}

pub fn pre_spawn_cube(
    world: &mut World,
    models: &HashMap<String, World>,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) {
    // Set the colour randomly
    let colour = if random() { Colour::Red } else { Colour::Blue };
    let model_name = match colour {
        Colour::Red => "Red Cube",
        Colour::Blue => "Blue Cube",
    };

    let cube = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();

    let rigid_body = RigidBodyBuilder::new_dynamic().lock_rotations().build();
    let handle = physics_context.rigid_bodies.insert(rigid_body);

    world.remove_one::<Visible>(cube).unwrap();
    world
        .insert(cube, (Cube {}, colour, RigidBody { handle }))
        .unwrap();
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
    Playing(Song),
    GameOver,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    pub track: MusicTrack,
    pub beat_length: Duration,
}
