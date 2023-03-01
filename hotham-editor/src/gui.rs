use glam::Vec2;
use hotham_editor_protocol::scene::EditorEntity;
use yakui::widgets::{List, Pad};
use yakui::{
    column, image, label, pad, row, slider, text, use_state, CrossAxisAlignment, TextureId,
};

pub fn gui(gui_state: &mut GuiState) {
    let scene = &gui_state.scene;
    let updates = &mut gui_state.updates;
    row(|| {
        column(|| {
            image(gui_state.texture_id, Vec2::new(500., 500.));
        });
        pad(Pad::all(20.0), || {
            let mut column = List::column();
            column.cross_axis_alignment = CrossAxisAlignment::Start;

            column.show(|| {
                text(42., scene.name.clone());
                for entity in &scene.entities {
                    row(|| {
                        label("Name");
                    });
                    row(|| {
                        text(20., entity.name.clone());
                    });
                    row(|| {
                        label("Translation");
                    });
                    row(|| {
                        yakui::column(|| {
                            label("x");
                            let x = entity.transform.translation.x as f64;
                            let x_state = use_state(move || x);
                            label(x_state.get().to_string());

                            if let Some(new_x) = slider(x_state.get(), -5.0, 5.0).value {
                                let mut new_entity = entity.clone();
                                x_state.set(new_x);
                                new_entity.transform.translation.x = new_x as _;
                                updates.push(new_entity);
                            }
                        });
                    });
                }
            });
        });
    });
}

pub struct GuiState {
    pub texture_id: TextureId,
    pub scene: hotham_editor_protocol::scene::Scene,
    pub updates: Vec<EditorEntity>,
}
