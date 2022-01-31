use crate::{
    components::Cube,
    resources::{game_context::GameState, GameContext},
};

use super::BeatSaberQueries;
use hotham::{
    components::{Panel, Visible},
    gltf_loader::add_model_to_world,
    hecs::World,
    resources::{vulkan_context::VulkanContext, AudioContext, RenderContext},
};

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) {
    // Get next state
    if let Some(next_state) = run(queries, world, game_context, vulkan_context, render_context) {
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
            let track = *game_context.music_tracks.get("Main Menu").unwrap();
            audio_context.play_music_track(track);
        }
        (GameState::MainMenu, GameState::Playing(track)) => {
            // Change visibility - ignore errors.
            let _ = world.remove_one::<Visible>(game_context.pointer);
            let _ = world.remove_one::<Visible>(game_context.main_menu_panel);

            // Switch tracks
            audio_context.play_music_track(*track);
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
) -> Option<GameState> {
    match &game_context.state {
        GameState::Init => return Some(GameState::MainMenu),
        GameState::MainMenu => {
            let panel = world.get::<Panel>(game_context.main_menu_panel).unwrap();
            if let Some(button) = panel.buttons.iter().filter(|p| p.clicked_this_frame).next() {
                let track = *game_context.music_tracks.get(&button.text).unwrap();
                return Some(GameState::Playing(track));
            }
        }
        GameState::Playing(_) => {
            // Spawn a cube if necessary
            spawn_cube(world, game_context, vulkan_context, render_context)
        }
    }

    None
}

fn spawn_cube(
    world: &mut World,
    game_context: &mut GameContext,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
) {
    let cube = add_model_to_world(
        "Blue Cube",
        &game_context.models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();

    world.insert_one(cube, Cube {}).unwrap();
}

#[cfg(test)]
mod tests {

    use hotham::{
        components::{Collider, RigidBody},
        Engine,
    };

    use crate::components::Cube;

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

        let main_menu_music = audio_context.dummy_track();
        game_context
            .music_tracks
            .insert("Main Menu".to_string(), main_menu_music);

        let beethoven = audio_context.dummy_track();
        game_context
            .music_tracks
            .insert("Beethoven - Op. 131".to_string(), beethoven);

        // INIT -> MAIN_MENU
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            audio_context,
            vulkan_context,
            render_context,
        );
        assert_eq!(game_context.state, GameState::MainMenu);
        assert!(world.get::<Visible>(game_context.pointer).is_ok());
        assert!(world.get::<Visible>(game_context.main_menu_panel).is_ok());
        assert_eq!(audio_context.current_music_track.unwrap(), main_menu_music);

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
        );
        assert_eq!(game_context.state, GameState::Playing(beethoven));
        assert_eq!(audio_context.current_music_track, Some(beethoven));
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
        );

        // Did we spawn a cube?
        let mut query = world.query::<(&Cube, &Visible, &RigidBody, &Collider)>();
        let mut i = query.iter();
        assert_eq!(i.len(), 1);
        let cube = i.next().unwrap();
    }
}
