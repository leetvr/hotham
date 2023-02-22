use glam::Vec2;
use yakui::TextureId;
use yakui::{column, image, label, row, text, widgets::Text, Color};

pub fn gui(gui_state: &GuiState) {
    row(|| {
        column(|| {
            image(gui_state.texture_id, Vec2::new(1500.0, 1500.0));
        });
        column(|| {
            text(42., gui_state.scene_name.clone());
        });
    });
}

pub struct GuiState {
    pub texture_id: TextureId,
    pub scene_name: String,
}
