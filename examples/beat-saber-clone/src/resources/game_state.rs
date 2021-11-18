#[derive(Debug, Clone, PartialEq)]
pub struct GameState {
    pub current_score: usize, // 0 means game over
}

impl Default for GameState {
    fn default() -> Self {
        Self { current_score: 1 }
    }
}
