use crate::resources::{game_context::GameState, GameContext};

use super::BeatSaberQueries;
use hotham::hecs::World;

pub fn game_system(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
) {
    // Get next state
    let current_state = game_context.state.clone();

    let next_state = get_next_state(queries, world, game_context);
    if next_state == current_state {
        // Nothing to do.
        return;
    };

    // If state has changed, transition
    transition(queries, world, game_context, current_state, next_state);
}

fn transition(
    queries: &mut BeatSaberQueries,
    world: &mut World,
    game_context: &mut GameContext,
    current_state: GameState,
    next_state: GameState,
) {
    match (&current_state, &next_state) {
        (GameState::Init, GameState::MainMenu) => {}
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
    if game_context.state == GameState::Init {
        return GameState::MainMenu;
    }

    todo!();
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut queries = Default::default();
        let mut world = Default::default();
        let mut game_context = Default::default();

        // INIT -> MAIN_MENU
        game_system(&mut queries, &mut world, &mut game_context);
        assert_eq!(game_context.state, GameState::MainMenu);
    }
}
