use hotham::hecs::Entity;

#[derive(Debug, Clone, PartialEq)]
pub struct GameContext {
    pub current_score: usize,
    pub state: GameState,
    pub pointer: Entity,
    pub main_menu_panel: Entity,
}

impl GameContext {
    pub fn new(pointer: Entity, main_menu_panel: Entity) -> Self {
        Self {
            current_score: 0,
            state: GameState::Init,
            pointer,
            main_menu_panel,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
}
