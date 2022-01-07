use hotham::components::Panel;
use legion::system;

use crate::resources::GameState;

#[system(for_each)]
pub fn update_ui(panel: &mut Panel, #[resource] game_state: &GameState) {
    let score = game_state.current_score;
    let new_string = format!("Current score: {}", score);
    panel.text = new_string;
}
