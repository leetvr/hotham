use std::collections::HashMap;

use hotham::{hecs::Entity, resources::audio_context::MusicTrack};

#[derive(Debug, Clone, PartialEq)]
pub struct GameContext {
    pub current_score: usize,
    pub state: GameState,
    pub pointer: Entity,
    pub main_menu_panel: Entity,
    pub music_tracks: HashMap<String, MusicTrack>,
}

impl GameContext {
    pub fn new(pointer: Entity, main_menu_panel: Entity) -> Self {
        Self {
            current_score: 0,
            state: GameState::Init,
            pointer,
            main_menu_panel,
            music_tracks: Default::default(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Init,
    MainMenu,
}
