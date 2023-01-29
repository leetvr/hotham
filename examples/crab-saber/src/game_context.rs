use std::{
    collections::HashMap,
    fmt::Debug,
    time::{Duration, Instant},
};

use hotham::{
    asset_importer::{self, add_model_to_world},
    components::{
        hand::Handedness,
        physics::{ActiveCollisionTypes, BodyType, SharedShape},
        ui_panel::add_ui_panel_to_world,
        Collider, GlobalTransform, LocalTransform, Pointer, RigidBody, SoundEmitter, Visible,
    },
    contexts::{audio_context::MusicTrack, physics_context::DEFAULT_COLLISION_GROUP, AudioContext},
    hecs::{Entity, World},
    vk, Engine,
};
use rand::prelude::*;

use crate::{
    components::{Color, Cube},
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
    pub fn new(engine: &mut Engine) -> Self {
        let render_context = &mut engine.render_context;
        let vulkan_context = &engine.vulkan_context;
        let gui_context = &engine.gui_context;
        let world = &mut engine.world;

        let glb_buffers: Vec<&[u8]> = vec![include_bytes!("../assets/crab_saber.glb")];
        let models =
            asset_importer::load_models_from_glb(&glb_buffers, vulkan_context, render_context)
                .expect("Unable to load models!");

        // Add the environment models
        add_environment(&models, world);

        // Add sabers
        let sabers = [Color::Blue, Color::Red].map(|color| add_saber(color, &models, world));

        // Spawn cubes
        for _ in 0..30 {
            pre_spawn_cube(world, &models);
        }

        // Add a pointer to let the player interact with the UI
        let pointer = add_pointer(&models, world);

        // Add a "backstop" collider to detect when a cube was missed
        let backstop = add_backstop(world);

        // Add UI panels
        let main_menu_panel = add_ui_panel_to_world(
            "Main Menu",
            vk::Extent2D {
                width: 800,
                height: 800,
            },
            [1.0, 1.0].into(),
            [0., 1., -1.].into(),
            vec![],
            vulkan_context,
            render_context,
            gui_context,
            world,
        );

        // Add panels
        let score_panel = add_ui_panel_to_world(
            "Current Score: 0",
            vk::Extent2D {
                width: 300,
                height: 600,
            },
            [0.5, 1.0].into(),
            [-1.25, 1., -2.].into(),
            vec![],
            vulkan_context,
            render_context,
            gui_context,
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
            last_spawn_time: Instant::now().checked_sub(Duration::new(100, 0)).unwrap(),
            sound_effects: Default::default(),
        }
    }

    pub fn add_songs(&mut self, audio_context: &mut AudioContext) {
        let main_menu_mp3 = include_bytes!("../assets/TrackTribe - Cloud Echo.mp3").to_vec();
        self.songs.insert(
            "Main Menu".to_string(),
            Song {
                beat_length: Duration::new(0, 0),
                track: audio_context.add_music_track(main_menu_mp3),
            },
        );

        let game_over_mp3 = include_bytes!("../assets/Chasms - Dark Matter.mp3").to_vec();
        self.songs.insert(
            "Game Over".to_string(),
            Song {
                beat_length: Duration::new(0, 0),
                track: audio_context.add_music_track(game_over_mp3),
            },
        );

        let right_here_beside_you =
            include_bytes!("../assets/Spence - Right Here Beside You.mp3").to_vec();
        self.songs.insert(
            "Spence - Right Here Beside You".to_string(),
            Song {
                beat_length: Duration::from_millis(60_000 / 129),
                track: audio_context.add_music_track(right_here_beside_you),
            },
        );

        let tell_me_that_i_cant =
            include_bytes!("../assets/NEFFEX - Tell Me That I Can't.mp3").to_vec();
        self.songs.insert(
            "NEFFEX - Tell Me That I Can't".to_string(),
            Song {
                beat_length: Duration::from_millis(60_000 / 70),
                track: audio_context.add_music_track(tell_me_that_i_cant),
            },
        );
    }

    pub fn add_sound_effects(&mut self, audio_context: &mut AudioContext) {
        let hit_mp3 = include_bytes!("../assets/Hit.mp3").to_vec();
        self.sound_effects.insert(
            "Hit".to_string(),
            audio_context.create_sound_emitter(hit_mp3),
        );

        let miss_mp3 = include_bytes!("../assets/Miss.mp3").to_vec();
        self.sound_effects.insert(
            "Miss".to_string(),
            audio_context.create_sound_emitter(miss_mp3),
        );
    }
}

fn add_backstop(world: &mut World) -> Entity {
    let collider = Collider {
        shape: SharedShape::cuboid(1., 1., 0.1),
        sensor: true,
        collision_groups: DEFAULT_COLLISION_GROUP,
        collision_filter: DEFAULT_COLLISION_GROUP,
        active_collision_types: ActiveCollisionTypes::all(),
        ..Default::default()
    };

    world.spawn((collider,))
}

fn add_pointer(models: &std::collections::HashMap<String, World>, world: &mut World) -> Entity {
    let pointer = add_model_to_world("Blue Pointer", models, world, None).unwrap();

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

fn add_environment(models: &std::collections::HashMap<String, World>, world: &mut World) {
    add_model_to_world("Environment", models, world, None);
    add_model_to_world("Ramp", models, world, None);
}

pub fn pre_spawn_cube(world: &mut World, models: &HashMap<String, World>) {
    // Set the color randomly
    let color = if random() { Color::Red } else { Color::Blue };
    let model_name = match color {
        Color::Red => "Red Cube",
        Color::Blue => "Blue Cube",
    };

    let cube = add_model_to_world(model_name, models, world, None).unwrap();
    let local_transform = LocalTransform {
        translation: [0., -100., 0.].into(),
        ..Default::default()
    };

    world.remove_one::<Visible>(cube).unwrap();
    world
        .insert(
            cube,
            (
                Cube {},
                color,
                RigidBody {
                    body_type: BodyType::KinematicVelocityBased,
                    lock_rotations: true,
                    linear_velocity: [0., 0., 0.].into(),
                    ..Default::default()
                },
                Collider {
                    sensor: true,
                    active_collision_types: ActiveCollisionTypes::all(),
                    shape: SharedShape::cuboid(0.2, 0.2, 0.2),
                    offset_from_parent: [0., 0.2, 0.].into(),
                    ..Default::default()
                },
                local_transform,
                GlobalTransform::from(local_transform),
            ),
        )
        .unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameState {
    Init,
    MainMenu,
    Playing(Song),
    GameOver,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Song {
    pub track: MusicTrack,
    pub beat_length: Duration,
}
