use std::time::{Duration, Instant};

use crate::{
    components::{Colour, Cube},
    resources::{
        game_context::{GameState, Song},
        GameContext,
    },
};

use super::BeatSaberQueries;
use hotham::{
    components::{
        hand::Handedness, panel::PanelButton, sound_emitter::SoundState, Collider, Info, Panel,
        RigidBody, Visible,
    },
    hecs::{Entity, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, InteractionGroups},
    resources::{
        physics_context::DEFAULT_COLLISION_GROUP, AudioContext, HapticContext, PhysicsContext,
    },
};
use rand::prelude::*;

const CUBE_X_OFFSETS: [f32; 4] = [-0.6, -0.2, 0.2, 0.6];
const CUBE_Y: f32 = 1.1;
const CUBE_Z: f32 = -10.;

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    physics_context: &mut PhysicsContext,
    haptic_context: &mut HapticContext,
) {
    // Get next state
    if let Some(next_state) = run(
        queries,
        world,
        game_context,
        audio_context,
        physics_context,
        haptic_context,
    ) {
        // If state has changed, transition
        transition(
            queries,
            world,
            game_context,
            audio_context,
            physics_context,
            next_state,
        );
    };
}

fn transition(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    physics_context: &mut PhysicsContext,
    next_state: GameState,
) {
    let current_state = &game_context.state;
    match (current_state, &next_state) {
        (GameState::Init | GameState::GameOver, GameState::MainMenu) => {
            // Make visible
            world.insert_one(game_context.pointer, Visible {}).unwrap();
            world
                .insert_one(game_context.main_menu_panel, Visible {})
                .unwrap();

            // Remove visibility
            // let _ = world.remove_one::<Visible>(game_context.score_panel);
            let _ = world.remove_one::<Visible>(game_context.blue_saber);
            let _ = world.remove_one::<Visible>(game_context.red_saber);

            // Switch tracks
            let song = game_context.songs.get("Main Menu").unwrap();
            audio_context.play_music_track(song.track);

            // Set panel text
            let mut panel = world
                .get_mut::<Panel>(game_context.main_menu_panel)
                .unwrap();

            panel.text = "CRAB SABER".to_string();
            panel.buttons = game_context
                .songs
                .iter()
                .filter_map(|(title, song)| {
                    if song.beat_length.as_millis() > 0 {
                        Some(PanelButton::new(title))
                    } else {
                        None
                    }
                })
                .collect();

            // Reset spawn time
            game_context.last_spawn_time -= Duration::new(100, 0);
        }
        (GameState::MainMenu, GameState::Playing(song)) => {
            // Reset score
            game_context.current_score = 0;

            // Make visible
            world
                .insert_one(game_context.score_panel, Visible {})
                .unwrap();
            world
                .insert_one(game_context.blue_saber, Visible {})
                .unwrap();
            world
                .insert_one(game_context.red_saber, Visible {})
                .unwrap();

            // Remove visibility
            let _ = world.remove_one::<Visible>(game_context.pointer);
            let _ = world.remove_one::<Visible>(game_context.main_menu_panel);

            // Switch tracks
            audio_context.play_music_track(song.track);
        }
        (GameState::Playing(_), GameState::GameOver) => {
            // Make visible
            world.insert_one(game_context.pointer, Visible {}).unwrap();
            world
                .insert_one(game_context.main_menu_panel, Visible {})
                .unwrap();

            // Make invisible
            let _ = world.remove_one::<Visible>(game_context.score_panel);
            let _ = world.remove_one::<Visible>(game_context.blue_saber);
            let _ = world.remove_one::<Visible>(game_context.red_saber);

            // Destroy all cubes
            let live_cubes = queries
                .live_cubes_query
                .query(world)
                .iter()
                .map(|(e, _)| e.clone())
                .collect::<Vec<_>>();
            dispose_of_cubes(live_cubes, world, physics_context);

            // Switch tracks
            let song = game_context.songs.get("Game Over").unwrap();
            audio_context.play_music_track(song.track);

            // Set panel text and add "OK" button
            let message = if game_context.current_score > 0 {
                "You did adequately!"
            } else {
                "YOU FAILED!"
            };
            let mut panel = world
                .get_mut::<Panel>(game_context.main_menu_panel)
                .unwrap();

            panel.text = format!("Game Over\n{}", message);
            panel.buttons = vec![PanelButton::new("Back to main menu")];
        }
        _ => panic!(
            "Invalid state transition {:?} -> {:?}",
            current_state, next_state
        ),
    }

    println!(
        "[BEAT_SABER] TRANSITION {:?} -> {:?}",
        current_state, next_state
    );

    game_context.state = next_state;
}

fn run(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    physics_context: &mut PhysicsContext,
    haptic_context: &mut HapticContext,
) -> Option<GameState> {
    match &mut game_context.state {
        GameState::Init => return Some(GameState::MainMenu),
        GameState::MainMenu => {
            let panel = world.get::<Panel>(game_context.main_menu_panel).unwrap();
            if let Some(button) = panel.buttons.iter().filter(|p| p.clicked_this_frame).next() {
                let song = game_context.songs.get(&button.text).unwrap();
                return Some(GameState::Playing(song.clone()));
            }
        }
        GameState::Playing(song) => {
            spawn_cube(
                queries,
                world,
                physics_context,
                song,
                &mut game_context.last_spawn_time,
            );

            check_for_hits(world, game_context, physics_context, haptic_context);
            update_panel_text(world, game_context);

            if game_context.current_score < 0
                || audio_context.music_track_status() == SoundState::Stopped
            {
                return Some(GameState::GameOver);
            };
        }
        GameState::GameOver => {
            if world
                .get::<Panel>(game_context.main_menu_panel)
                .unwrap()
                .buttons[0]
                .clicked_this_frame
            {
                return Some(GameState::MainMenu);
            }
        }
    }

    None
}

fn spawn_cube(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    physics_context: &mut PhysicsContext,
    song: &mut Song,
    last_spawn_time: &mut Instant,
) {
    if !should_spawn_cube(*last_spawn_time, song.beat_length) {
        return;
    }

    println!("[BEAT_SABER] Spawning cube!");
    let colour = if random() { Colour::Red } else { Colour::Blue };
    let dead_cube = queries
        .dead_cubes_query
        .query_mut(world)
        .find_map(|(e, c)| if c == &colour { Some(e) } else { None })
        .unwrap();
    revive_cube(dead_cube, world, physics_context, song);
    *last_spawn_time = Instant::now();
}

fn update_panel_text(world: &mut World, game_context: &mut GameContext) {
    world
        .get_mut::<Panel>(game_context.score_panel)
        .unwrap()
        .text = format!("Score: {}", game_context.current_score);
}

fn check_for_hits(
    world: &mut World,
    game_context: &mut GameContext,
    physics_context: &mut PhysicsContext,
    haptic_context: &mut HapticContext,
) {
    let mut pending_sound_effects = Vec::new();
    let mut cubes_to_dispose = Vec::new();

    {
        let blue_saber_collider = world.get::<Collider>(game_context.blue_saber).unwrap();
        for c in &blue_saber_collider.collisions_this_frame {
            let e = world.entity(*c).unwrap();
            if !is_cube(e) {
                continue;
            };
            if let Some(colour) = e.get::<Colour>() {
                match *colour {
                    Colour::Red => {
                        game_context.current_score -= 1;
                        pending_sound_effects.push((c.clone(), "Miss"));
                    }
                    Colour::Blue => {
                        game_context.current_score += 1;
                        pending_sound_effects.push((c.clone(), "Hit"));
                    }
                }
                haptic_context.request_haptic_feedback(1., Handedness::Right);
                cubes_to_dispose.push(c.clone());
            }
        }

        let red_saber_collider = world.get::<Collider>(game_context.red_saber).unwrap();
        for c in &red_saber_collider.collisions_this_frame {
            let e = world.entity(*c).unwrap();
            if !is_cube(e) {
                continue;
            };
            if let Some(colour) = e.get::<Colour>() {
                match *colour {
                    Colour::Red => {
                        game_context.current_score += 1;
                        pending_sound_effects.push((c.clone(), "Hit"));
                    }
                    Colour::Blue => {
                        game_context.current_score -= 1;
                        pending_sound_effects.push((c.clone(), "Miss"));
                    }
                }
                haptic_context.request_haptic_feedback(1., Handedness::Left);
                cubes_to_dispose.push(c.clone());
            }
        }

        let backstop_collider = world.get::<Collider>(game_context.backstop).unwrap();
        for c in &backstop_collider.collisions_this_frame {
            let e = world.entity(*c).unwrap();
            if !is_cube(e) {
                continue;
            };
            if e.get::<Cube>().is_some() {
                game_context.current_score -= 1;
                pending_sound_effects.push((c.clone(), "Miss"));
                cubes_to_dispose.push(c.clone());
            }
        }
    }

    play_sound_effects(pending_sound_effects, world, game_context);
    dispose_of_cubes(cubes_to_dispose, world, physics_context);
}

fn is_cube(e: hotham::hecs::EntityRef) -> bool {
    e.has::<Cube>() && e.has::<Visible>() && e.has::<Collider>() && e.has::<RigidBody>()
}

fn dispose_of_cubes(
    cubes_to_dispose: Vec<Entity>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    for e in cubes_to_dispose.into_iter() {
        println!("[BEAT_SABER] Disposing of cube {:?}", e);
        match world.get::<Collider>(e) {
            Ok(c) => {
                let handle = c.handle;
                physics_context.colliders.remove(
                    handle,
                    &mut physics_context.island_manager,
                    &mut physics_context.rigid_bodies,
                    false,
                );
            }
            Err(_) => {
                let info = world.get::<Info>(e).unwrap();
                println!("Unable to find collider for entity {:?} - {:?}", e, *info);
            }
        }
        drop(world.remove::<(Collider, Visible)>(e));
    }
}

fn play_sound_effects(
    pending_effects: Vec<(Entity, &'static str)>,
    world: &mut World,
    game_context: &GameContext,
) {
    for (entity, effect_name) in pending_effects.into_iter() {
        let mut effect = game_context.sound_effects.get(effect_name).unwrap().clone();
        effect.play();
        world.insert_one(entity, effect).unwrap()
    }
}

fn should_spawn_cube(last_spawn_time: Instant, beat_length: Duration) -> bool {
    let delta = Instant::now() - last_spawn_time;
    delta > beat_length
}

fn revive_cube(
    cube_entity: Entity,
    world: &mut World,
    physics_context: &mut PhysicsContext,
    song: &Song,
) {
    println!("[BEAT_SABER] Reviving dead cube - {:?}", cube_entity);
    let mut rng = thread_rng();
    let translation_x = CUBE_X_OFFSETS[rng.gen_range(0..4)];
    let z_linvel = -CUBE_Z / (song.beat_length.as_secs_f32() * 4.); // distance / time for 4 beats

    // Update the Rigid Body
    let rigid_body_handle = world.get::<RigidBody>(cube_entity).unwrap().handle;
    let rigid_body = &mut physics_context.rigid_bodies[rigid_body_handle];
    rigid_body.set_translation([translation_x, CUBE_Y, CUBE_Z].into(), true);
    rigid_body.set_linvel([0., 0., z_linvel].into(), true);

    // Create a new collider
    let collider = ColliderBuilder::cuboid(0.2, 0.2, 0.2)
        .translation([0., 0.2, 0.].into()) // Offset the collider slightly
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .collision_groups(InteractionGroups::new(
            DEFAULT_COLLISION_GROUP,
            DEFAULT_COLLISION_GROUP,
        ))
        .user_data(cube_entity.id() as _)
        .build();

    // Attach it to the rigidbody
    let collider_handle = physics_context.colliders.insert_with_parent(
        collider,
        rigid_body_handle,
        &mut physics_context.rigid_bodies,
    );

    world
        .insert(cube_entity, (Visible {}, Collider::new(collider_handle)))
        .unwrap();
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {

    use std::time::Duration;

    use hotham::{
        components::{Collider, RigidBody, SoundEmitter},
        hecs::Entity,
        nalgebra::Vector3,
        rapier3d::prelude::RigidBodyBuilder,
        resources::HapticContext,
        Engine,
    };

    use super::*;
    use crate::{components::Cube, resources::game_context::Song};

    #[test]
    pub fn game_system_test() {
        let mut engine = Engine::new();
        let mut queries = Default::default();
        let mut world = World::new();
        let mut game_context = GameContext::new(&mut engine, &mut world);
        let audio_context = &mut engine.audio_context;
        let physics_context = &mut engine.physics_context;
        let haptic_context = &mut engine.haptic_context;
        let world = &mut world;
        let game_context = &mut game_context;

        let main_menu_music = audio_context.dummy_track();
        let main_menu_music = Song {
            beat_length: Duration::new(0, 0),
            track: main_menu_music,
        };

        game_context
            .songs
            .insert("Main Menu".to_string(), main_menu_music.clone());

        let game_over_music = audio_context.dummy_track();
        let game_over_music = Song {
            beat_length: Duration::from_millis(0),
            track: game_over_music,
        };
        game_context
            .songs
            .insert("Game Over".to_string(), game_over_music.clone());

        let beside_you = audio_context.dummy_track();
        let beside_you = Song {
            beat_length: Duration::from_millis(500),
            track: beside_you,
        };
        game_context.songs.insert(
            "Spence - Right Here Beside You".to_string(),
            beside_you.clone(),
        );

        game_context
            .sound_effects
            .insert("Hit".to_string(), audio_context.dummy_sound_emitter());
        game_context
            .sound_effects
            .insert("Miss".to_string(), audio_context.dummy_sound_emitter());

        // INIT -> MAIN_MENU
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        assert_eq!(game_context.state, GameState::MainMenu);
        assert!(is_visible(world, game_context.pointer));
        assert!(is_visible(world, game_context.main_menu_panel));
        assert!(!is_visible(world, game_context.blue_saber));
        assert!(!is_visible(world, game_context.red_saber));
        assert!(!is_visible(world, game_context.score_panel));
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
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        assert_eq!(game_context.state, GameState::Playing(beside_you.clone()));
        assert_eq!(audio_context.current_music_track, Some(beside_you.track));
        assert!(!is_visible(world, game_context.pointer));
        assert!(!is_visible(world, game_context.main_menu_panel));
        assert!(is_visible(world, game_context.blue_saber));
        assert!(is_visible(world, game_context.red_saber));
        assert!(is_visible(world, game_context.score_panel));

        // PLAYING - TICK ONE
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );

        {
            let mut q = queries.live_cubes_query.query(world);
            let mut i = q.iter();
            assert_eq!(i.len(), 1);
            let (_, (_, rigid_body, _)) = i.next().unwrap();
            let rigid_body = &physics_context.rigid_bodies[rigid_body.handle];
            drop(q);

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
            assert_score_is(world, game_context, 0);
        }

        // PLAYING - TICK TWO
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );

        {
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 0);

            // Simulate blue saber hitting blue cube - increase score
            hit_cube(
                game_context.blue_saber,
                Colour::Blue,
                world,
                physics_context,
            );
        }

        // PLAYING - TICK THREE
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );

        {
            assert_cube_processed(world, game_context.blue_saber, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 1);
            // Simulate blue saber hitting red cube - decrease score
            hit_cube(game_context.blue_saber, Colour::Red, world, physics_context);
            // Reset spawn timer.
            game_context.last_spawn_time =
                Instant::now() - beside_you.beat_length - Duration::from_millis(1);
        }

        // PLAYING - TICK FOUR
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_cube_processed(world, game_context.blue_saber, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 0);
            assert_eq!(num_cubes(world), 2);

            // Simulate blue saber hitting blue cube - increase score
            hit_cube(
                game_context.blue_saber,
                Colour::Blue,
                world,
                physics_context,
            );

            // Make the sabers collide
            collide_sabers(game_context, world);
        }

        // PLAYING - TICK FIVE
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_cube_processed(world, game_context.blue_saber, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 1);
            // Simulate blue cube hitting the backstop - decrease score
            hit_cube(game_context.backstop, Colour::Blue, world, physics_context);
        }

        // PLAYING - TICK SIX
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_cube_processed(world, game_context.backstop, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 0);

            // Add a red cube to the red saber - increase score
            hit_cube(game_context.red_saber, Colour::Red, world, physics_context);
        }

        // PLAYING - TICK SEVEN
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_cube_processed(world, game_context.red_saber, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 1);
            // Add a blue cube to the red saber - decrease score
            hit_cube(game_context.red_saber, Colour::Blue, world, physics_context);
        }

        // PLAYING - TICK EIGHT
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_cube_processed(world, game_context.red_saber, haptic_context);
            reset(world, game_context, haptic_context);
            assert_score_is(world, game_context, 0);
            // Add a blue cube to the red saber - decrease score
            hit_cube(game_context.red_saber, Colour::Blue, world, physics_context);
        }

        // PLAYING - TICK NINE -> GAME OVER
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_eq!(game_context.state, GameState::GameOver);
            assert!(is_visible(world, game_context.pointer));
            assert!(is_visible(world, game_context.main_menu_panel));
            assert!(!is_visible(world, game_context.blue_saber));
            assert!(!is_visible(world, game_context.red_saber));
            assert!(!is_visible(world, game_context.score_panel));
            assert_eq!(
                audio_context.current_music_track.unwrap(),
                game_over_music.track
            );
            assert_eq!(num_cubes(world), 0);

            let mut panel = world
                .get_mut::<Panel>(game_context.main_menu_panel)
                .unwrap();
            assert_eq!(panel.text, "Game Over\nYOU FAILED!",);
            assert_eq!(panel.buttons[0].text, "Back to main menu",);
            panel.buttons[0].clicked_this_frame = true;
        }

        // GAME_OVER -> MAIN_MENU
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        {
            assert_eq!(game_context.state, GameState::MainMenu);
            assert!(is_visible(world, game_context.pointer));
            assert!(is_visible(world, game_context.main_menu_panel));
            assert!(!is_visible(world, game_context.blue_saber));
            assert!(!is_visible(world, game_context.red_saber));
            assert!(!is_visible(world, game_context.score_panel));
            assert_eq!(
                audio_context.current_music_track.unwrap(),
                main_menu_music.track
            );
            assert_eq!(
                &world
                    .get::<Panel>(game_context.main_menu_panel)
                    .unwrap()
                    .text,
                "Main Menu",
            );
            assert_eq!(
                &world
                    .get::<Panel>(game_context.main_menu_panel)
                    .unwrap()
                    .buttons[0]
                    .text,
                "Spence - Right Here Beside You",
            );
        }

        // MAIN_MENU -> PLAYING
        {
            let mut panel = world
                .get_mut::<Panel>(game_context.main_menu_panel)
                .unwrap();
            panel.buttons[0].clicked_this_frame = true;
        }
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        reset(world, game_context, haptic_context);
        assert_eq!(game_context.current_score, 0);
        assert_eq!(game_context.state, GameState::Playing(beside_you.clone()));
        assert_eq!(audio_context.current_music_track, Some(beside_you.track));
        assert!(!is_visible(world, game_context.pointer));
        assert!(!is_visible(world, game_context.main_menu_panel));
        assert!(is_visible(world, game_context.blue_saber));
        assert!(is_visible(world, game_context.red_saber));
        assert!(is_visible(world, game_context.score_panel));

        // PLAYING - TICK ONE
        game_system(
            &mut queries,
            world,
            game_context,
            audio_context,
            physics_context,
            haptic_context,
        );
        assert_eq!(num_cubes(world), 1);
    }

    fn collide_sabers(game_context: &mut GameContext, world: &mut World) {
        world
            .get_mut::<Collider>(game_context.blue_saber)
            .unwrap()
            .collisions_this_frame
            .push(game_context.red_saber.clone());
        world
            .get_mut::<Collider>(game_context.red_saber)
            .unwrap()
            .collisions_this_frame
            .push(game_context.blue_saber.clone());
    }

    fn num_cubes(world: &mut World) -> usize {
        world
            .query::<(&Colour, &Cube, &Visible, &RigidBody, &Collider)>()
            .iter()
            .len()
    }

    fn hit_cube(
        saber: Entity,
        colour: Colour,
        world: &mut World,
        physics_context: &mut PhysicsContext,
    ) {
        let rigid_body = physics_context
            .rigid_bodies
            .insert(RigidBodyBuilder::new_dynamic().build());
        let collider = physics_context
            .colliders
            .insert(ColliderBuilder::cuboid(0., 0., 0.).build());
        let cube = world.spawn((
            colour,
            Cube {},
            Visible {},
            RigidBody { handle: rigid_body },
            Collider::new(collider),
        ));
        println!("[TEST] Adding dummy cube: {:?}", cube);
        world
            .get_mut::<Collider>(saber)
            .unwrap()
            .collisions_this_frame
            .push(cube);
    }

    fn assert_cube_processed(world: &mut World, saber: Entity, haptic_context: &mut HapticContext) {
        let hit_cube = world.get::<Collider>(saber).unwrap().collisions_this_frame[0];
        let hit_cube = world.entity(hit_cube).unwrap();
        assert!(hit_cube.has::<SoundEmitter>());
        assert!(hit_cube.has::<RigidBody>()); // Necessary to play a sound
        assert!(!hit_cube.has::<Visible>());
        assert!(!hit_cube.has::<Collider>());

        if let Ok(c) = world.get::<Colour>(saber) {
            match *c {
                Colour::Red => assert_eq!(haptic_context.left_hand_amplitude_this_frame, 1.),
                Colour::Blue => assert_eq!(haptic_context.right_hand_amplitude_this_frame, 1.),
            }
        }
    }

    pub fn reset(
        world: &mut World,
        game_context: &mut GameContext,
        haptic_context: &mut HapticContext,
    ) {
        world
            .get_mut::<Collider>(game_context.red_saber)
            .unwrap()
            .collisions_this_frame = vec![];
        world
            .get_mut::<Collider>(game_context.blue_saber)
            .unwrap()
            .collisions_this_frame = vec![];
        world
            .get_mut::<Collider>(game_context.backstop)
            .unwrap()
            .collisions_this_frame = vec![];

        haptic_context.right_hand_amplitude_this_frame = 0.;
        haptic_context.left_hand_amplitude_this_frame = 0.;
    }

    pub fn is_visible(world: &World, entity: Entity) -> bool {
        world.get::<Visible>(entity).is_ok()
    }

    pub fn assert_score_is(world: &mut World, game_context: &mut GameContext, score: i32) {
        assert_eq!(game_context.current_score, score);
        assert_eq!(
            world.get::<Panel>(game_context.score_panel).unwrap().text,
            format!("Score: {}", score)
        );
    }
}
