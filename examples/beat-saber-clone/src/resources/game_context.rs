#[derive(Debug, Clone, PartialEq)]
pub struct GameContext {
    pub current_score: usize,
    pub state: GameState,
}

impl Default for GameContext {
    fn default() -> Self {
        Self {
            current_score: 0,
            state: GameState::Init,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
}
