use crate::resources::{game_context::GameState, GameContext};

use super::BeatSaberQueries;
use hotham::{components::Visible, hecs::World, resources::AudioContext};

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    audio_context: &mut AudioContext,
) {
    // Get next state
    let next_state = get_next_state(queries, world, game_context);
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
            world.insert_one(game_context.pointer, Visible {}).unwrap();
            world
                .insert_one(game_context.main_menu_panel, Visible {})
                .unwrap();
            let track = *audio_context.music_tracks.get("Main Menu").unwrap();
            audio_context.play_music_track(track);
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
) -> GameState {
    match &game_context.state {
        GameState::Init => GameState::MainMenu,
        GameState::MainMenu => GameState::MainMenu,
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut queries = Default::default();
        let mut world = World::new();
        let pointer = world.spawn(());
        let main_menu_panel = world.spawn(());

        let mut audio_context = AudioContext::default();
        let main_menu_music = audio_context.dummy_track();
        audio_context
            .music_tracks
            .insert("Main Menu".to_string(), main_menu_music);
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
    }
}
