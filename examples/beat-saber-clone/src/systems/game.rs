use std::collections::HashMap;

use crate::{
    components::Cube,
    resources::{
        game_context::{GameState, Song},
        GameContext,
    },
};

use super::BeatSaberQueries;
use hotham::{
    components::{Panel, Visible},
    gltf_loader::add_model_to_world,
    hecs::World,
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{vulkan_context::VulkanContext, AudioContext, PhysicsContext, RenderContext},
};
use rand::prelude::*;

const CUBE_X_OFFSETS: [f32; 4] = [-0.6, -0.2, 0.2, 0.6];
const CUBE_Y: f32 = 1.1;
const CUBE_Z: f32 = 10.;

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) {
    // Get next state
    if let Some(next_state) = run(
        queries,
        world,
        game_context,
        vulkan_context,
        render_context,
        physics_context,
    ) {
        // If state has changed, transition
        transition(queries, world, game_context, audio_context, next_state);
    };
}

fn transition(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    next_state: GameState,
) {
    let current_state = &game_context.state;
    match (current_state, &next_state) {
        (GameState::Init, GameState::MainMenu) => {
            // Change visibility
            world.insert_one(game_context.pointer, Visible {}).unwrap();
            world
                .insert_one(game_context.main_menu_panel, Visible {})
                .unwrap();

            // Switch tracks
            let song = game_context.songs.get("Main Menu").unwrap();
            audio_context.play_music_track(song.track);
        }
        (GameState::MainMenu, GameState::Playing(song)) => {
            // Change visibility - ignore errors.
            let _ = world.remove_one::<Visible>(game_context.pointer);
            let _ = world.remove_one::<Visible>(game_context.main_menu_panel);

            // Switch tracks
            audio_context.play_music_track(song.track);
        }
        _ => panic!(
            "Invalid state transition {:?} -> {:?}",
            current_state, next_state
        ),
    }

    game_context.state = next_state;
}

fn run(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) -> Option<GameState> {
    match &game_context.state {
        GameState::Init => return Some(GameState::MainMenu),
        GameState::MainMenu => {
            let panel = world.get::<Panel>(game_context.main_menu_panel).unwrap();
            if let Some(button) = panel.buttons.iter().filter(|p| p.clicked_this_frame).next() {
                let song = game_context.songs.get(&button.text).unwrap();
                return Some(GameState::Playing(song.clone()));
            }
        }
        GameState::Playing(song) => {
            // Spawn a cube if necessary
            spawn_cube(
                world,
                &game_context.models,
                vulkan_context,
                render_context,
                physics_context,
                song,
            )
        }
    }

    None
}

fn spawn_cube(
    world: &mut World,
    models: &HashMap<String, World>,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
    song: &Song,
) {
    let cube = add_model_to_world(
        "Blue Cube",
        &models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();

    let mut rng = thread_rng();
    let translation_x = CUBE_X_OFFSETS[rng.gen_range(0..4)];
    let z_linvel = CUBE_Z / (song.beat_length * 4.); // distance / time for 4 beats

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cuboid(0.2, 0.2, 0.2)
        .translation([0., 0.2, 0.].into()) // Offset the collider slightly
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_dynamic()
        .translation([translation_x, CUBE_Y, CUBE_Z].into())
        .linvel([0., 0., z_linvel].into())
        .lock_rotations()
        .build();
    let (collider, rigid_body) =
        physics_context.get_rigid_body_and_collider(cube, rigid_body, collider);

    world.insert(cube, (Cube {}, collider, rigid_body)).unwrap();
}

#[cfg(test)]
mod tests {

    use hotham::{
        components::{Collider, RigidBody},
        nalgebra::Vector3,
        Engine,
    };

    use crate::{components::Cube, resources::game_context::Song};

    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut engine = Engine::new();
        let mut queries = Default::default();
        let mut world = World::new();
        let mut game_context = GameContext::new(&mut engine, &mut world);
        let audio_context = &mut engine.audio_context;
        let vulkan_context = &engine.vulkan_context;
        let render_context = &engine.render_context;
        let physics_context = &mut engine.physics_context;

        let main_menu_music = audio_context.dummy_track();
        let main_menu_music = Song {
            beat_length: 0.,
            track: main_menu_music,
        };
        game_context
            .songs
            .insert("Main Menu".to_string(), main_menu_music.clone());

        let beside_you = audio_context.dummy_track();
        let beside_you = Song {
            beat_length: 0.5,
            track: beside_you,
        };
        game_context.songs.insert(
            "Spence - Right Here Beside You".to_string(),
            beside_you.clone(),
        );

        // INIT -> MAIN_MENU
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            audio_context,
            vulkan_context,
            render_context,
            physics_context,
        );
        assert_eq!(game_context.state, GameState::MainMenu);
        assert!(world.get::<Visible>(game_context.pointer).is_ok());
        assert!(world.get::<Visible>(game_context.main_menu_panel).is_ok());
        assert_eq!(
            audio_context.current_music_track.unwrap(),
            main_menu_music.track
        );

        // MAIN_MENU -> PLAYING
        {
            let mut panel = world
                .get_mut::<Panel>(game_context.main_menu_panel)
                .unwrap();
            panel.buttons[0].clicked_this_frame = true;
        }
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            audio_context,
            vulkan_context,
            render_context,
            physics_context,
        );
        assert_eq!(game_context.state, GameState::Playing(beside_you.clone()));
        assert_eq!(audio_context.current_music_track, Some(beside_you.track));
        assert!(world.get::<Visible>(game_context.pointer).is_err());
        assert!(world.get::<Visible>(game_context.main_menu_panel).is_err());
        assert!(world.get::<Visible>(game_context.blue_saber).is_ok());
        assert!(world.get::<Visible>(game_context.red_saber).is_ok());

        // PLAYING - TICK ONE
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            audio_context,
            vulkan_context,
            render_context,
            physics_context,
        );

        // Did we spawn a cube?
        {
            let mut query = world.query::<(&Cube, &Visible, &RigidBody, &Collider)>();
            let mut i = query.iter();
            assert_eq!(i.len(), 1);
            let (_, (_, _, rigid_body, _)) = i.next().unwrap();
            let rigid_body = &physics_context.rigid_bodies[rigid_body.handle];
            let t = rigid_body.translation();
            assert!(
                t[0] == CUBE_X_OFFSETS[0]
                    || t[0] == CUBE_X_OFFSETS[1]
                    || t[0] == CUBE_X_OFFSETS[2]
                    || t[0] == CUBE_X_OFFSETS[3]
            );
            assert_eq!(t[1], CUBE_Y);
            assert_eq!(t[2], CUBE_Z);
            assert_eq!(rigid_body.linvel(), &Vector3::new(0., 0., 5.,));
        }

        // PLAYING - TICK TWO
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            audio_context,
            vulkan_context,
            render_context,
            physics_context,
        );

        {
            let mut query = world.query::<(&Cube, &Visible, &RigidBody, &Collider)>();
            let mut i = query.iter();
            assert_eq!(i.len(), 1);
        }
    }
}
