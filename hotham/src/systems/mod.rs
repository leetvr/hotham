#![allow(missing_docs)]
pub mod animation;
pub mod audio;
pub mod collision;
pub mod debug;
pub mod draw_gui;
pub mod grabbing;
pub mod hands;
pub mod pointers;
pub mod rendering;
pub mod skinning;
pub mod stage;
pub mod update_global_transform;
pub mod update_global_transform_with_parent;
pub mod update_local_transform_with_rigid_body;

pub use animation::animation_system;
pub use audio::audio_system;
pub use collision::collision_system;
pub use draw_gui::draw_gui_system;
pub use grabbing::grabbing_system;
pub use hands::hands_system;
pub use pointers::pointers_system;
pub use rendering::rendering_system;
pub use skinning::skinning_system;
pub use stage::{add_stage, update_views_with_stage_transform};
pub use update_global_transform::update_global_transform_system;
pub use update_global_transform_with_parent::update_global_transform_with_parent_system;
pub use update_local_transform_with_rigid_body::update_local_transform_with_rigid_body_system;

use crate::components::{
    AnimationController, Collider, GlobalTransform, Hand, Info, Joint, LocalTransform, Mesh, Panel,
    Parent, Pointer, RigidBody, Skin, SoundEmitter, UIPanel, Visible,
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
    pub joints_query: PreparedQuery<(&'a GlobalTransform, &'a Joint, &'a Info)>,
    pub skins_query: PreparedQuery<(&'a Skin, &'a GlobalTransform)>,
    pub parent_query: PreparedQuery<&'a Parent>,
    #[allow(clippy::type_complexity)]
    pub rendering_query:
        PreparedQuery<With<Visible, (&'a Mesh, &'a GlobalTransform, Option<&'a Skin>)>>,
    pub roots_query: PreparedQuery<Without<Parent, &'a GlobalTransform>>,
    pub update_rigid_body_transforms_query: PreparedQuery<(&'a RigidBody, &'a mut LocalTransform)>,
    pub update_global_transform_query: PreparedQuery<(&'a LocalTransform, &'a mut GlobalTransform)>,
    pub pointers_query: PreparedQuery<With<Visible, (&'a mut Pointer, &'a mut LocalTransform)>>,
}
