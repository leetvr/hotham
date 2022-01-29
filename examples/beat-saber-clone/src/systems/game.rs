use crate::resources::{game_context::GameState, GameContext};

use super::BeatSaberQueries;
use hotham::{
    components::{Panel, Visible},
    hecs::World,
    resources::AudioContext,
};

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
) {
    // Get next state
    let next_state = get_next_state(queries, world, game_context, audio_context);
    if next_state == game_context.state {
        // Nothing to do.
        return;
    };

    // If state has changed, transition
    transition(queries, world, game_context, audio_context, next_state);
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
            let track = *audio_context.music_tracks.get("Main Menu").unwrap();
            audio_context.play_music_track(track);
        }
        (GameState::MainMenu, GameState::Playing(track)) => {
            // Change visibility
            let _ = world.remove_one::<Visible>(game_context.pointer); // no need to be strict
            let _ = world.remove_one::<Visible>(game_context.main_menu_panel); // no need to be strict

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

fn get_next_state(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
) -> GameState {
    match &game_context.state {
        GameState::Init => GameState::MainMenu,
        GameState::MainMenu => {
            let panel = world.get::<Panel>(game_context.main_menu_panel).unwrap();
            if let Some(button) = panel.buttons.iter().filter(|p| p.clicked_this_frame).next() {
                let track = *audio_context.music_tracks.get(&button.text).unwrap();
                GameState::Playing(track)
            } else {
                GameState::MainMenu
            }
        }
        _ => todo!(),
    }
}

#[cfg(test)]
mod tests {

    use hotham::{
        components::panel::{create_panel, PanelButton},
        Engine,
    };

    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut engine = Engine::new();
        let mut queries = Default::default();
        let mut world = World::new();
        let mut audio_context = &mut engine.audio_context;
        let gui_context = &engine.gui_context;
        let vulkan_context = &engine.vulkan_context;
        let render_context = &engine.render_context;

        let pointer = world.spawn(());

        let components = create_panel(
            "Test",
            1,
            1,
            vulkan_context,
            render_context,
            gui_context,
            vec![PanelButton::new("Beethoven")],
        );
        let main_menu_panel = world.spawn(components);

        let main_menu_music = audio_context.dummy_track();
        let beethoven = audio_context.dummy_track();
        audio_context
            .music_tracks
            .insert("Main Menu".to_string(), main_menu_music);
        audio_context
            .music_tracks
            .insert("Beethoven".to_string(), beethoven);
        let mut game_context = GameContext::new(pointer, main_menu_panel);

        // INIT -> MAIN_MENU
        game_system(
            &mut queries,
            &mut world,
            &mut game_context,
            &mut audio_context,
        );
        assert_eq!(game_context.state, GameState::MainMenu);
        assert!(world.get::<Visible>(pointer).is_ok());
        assert!(world.get::<Visible>(main_menu_panel).is_ok());
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
            &mut audio_context,
        );
        assert_eq!(game_context.state, GameState::Playing(beethoven));
        assert_eq!(audio_context.current_music_track, Some(beethoven));
        assert!(world.get::<Visible>(pointer).is_err());
        assert!(world.get::<Visible>(main_menu_panel).is_err());
    }
}
