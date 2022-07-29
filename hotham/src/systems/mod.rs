#![allow(missing_docs)]
pub mod animation;
pub mod audio;
pub mod collision;
pub mod draw_gui;
pub mod grabbing;
pub mod hands;
pub mod pointers;
pub mod rendering;
pub mod room;
pub mod skinning;
pub mod update_parent_transform_matrix;
pub mod update_rigid_body_transforms;
pub mod update_transform_matrix;

pub use animation::animation_system;
pub use audio::audio_system;
pub use collision::collision_system;
pub use draw_gui::draw_gui_system;
pub use grabbing::grabbing_system;
pub use hands::hands_system;
pub use pointers::pointers_system;
pub use rendering::rendering_system;
pub use room::{add_room, update_views_with_room_transform};
pub use skinning::skinning_system;
pub use update_parent_transform_matrix::update_parent_transform_matrix_system;
pub use update_rigid_body_transforms::update_rigid_body_transforms_system;
pub use update_transform_matrix::update_transform_matrix_system;

use crate::components::{
    AnimationController, Collider, Hand, Info, Joint, Mesh, Panel, Parent, Pointer, RigidBody,
    Skin, SoundEmitter, Transform, TransformMatrix, UIPanel, Visible,
};
use hecs::{PreparedQuery, With, Without};

/// Queries used by `system`s in Hotham
#[derive(Default)]
pub struct Queries<'a> {
    pub animation_query: PreparedQuery<&'a AnimationController>,
    pub audio_query: PreparedQuery<(&'a mut SoundEmitter, &'a RigidBody)>,
    pub collision_query: PreparedQuery<&'a mut Collider>,
    pub draw_gui_query: PreparedQuery<(&'a mut Panel, &'a mut UIPanel)>,
    pub grabbing_query: PreparedQuery<(&'a mut Hand, &'a Collider)>,
    pub hands_query: PreparedQuery<(&'a mut Hand, &'a mut AnimationController, &'a mut RigidBody)>,
    pub joints_query: PreparedQuery<(&'a TransformMatrix, &'a Joint, &'a Info)>,
    pub skins_query: PreparedQuery<(&'a Skin, &'a TransformMatrix)>,
    pub parent_query: PreparedQuery<&'a Parent>,
    #[allow(clippy::type_complexity)]
    pub rendering_query: PreparedQuery<
        With<
            Visible,
            (
                &'a Mesh,
                &'a Transform,
                &'a TransformMatrix,
                Option<&'a Skin>,
            ),
        >,
    >,
    pub roots_query: PreparedQuery<Without<Parent, &'a TransformMatrix>>,
    pub update_rigid_body_transforms_query: PreparedQuery<(&'a RigidBody, &'a mut Transform)>,
    pub update_transform_matrix_query: PreparedQuery<(&'a Transform, &'a mut TransformMatrix)>,
    pub pointers_query: PreparedQuery<With<Visible, (&'a mut Pointer, &'a mut Transform)>>,
}
