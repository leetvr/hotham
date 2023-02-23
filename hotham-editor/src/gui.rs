use glam::Vec2;
use yakui::widgets::{List, Panel};
use yakui::{
    column, expanded, image, label, row, text, textbox, use_state, CrossAxisAlignment, TextureId,
};

pub fn gui(gui_state: GuiState) {
    let scene = gui_state.scene;
    row(|| {
        column(|| {
            image(gui_state.texture_id, Vec2::new(500., 500.));
        });
        column(|| {
            text(42., scene.name);
            expanded(|| {
                let panel = Panel::side();
                panel.show(|| {
                    let mut column = List::column();
                    column.cross_axis_alignment = CrossAxisAlignment::Start;

                    column.show(|| {
                        for entity in scene.entities {
                            row(|| {
                                text(20., "Name");
                            });
                            row(|| {
                                text(20., entity.name);
                            });
                            row(|| {
                                text(20., "Translation");
                            });
                            row(|| {
                                text(20., format!("{:?}", entity.transform.translation));
                            });
                        }

                        label("Input");
                        let name = use_state(|| String::from("Hello"));

                        let res = textbox(name.borrow().clone());
                        if let Some(new_name) = res.text.as_ref() {
                            name.set(new_name.clone());
                        }
                    });
                });
            });
        });
    });
}

pub struct GuiState {
    pub texture_id: TextureId,
    pub scene: hotham_editor_protocol::scene::Scene,
}
