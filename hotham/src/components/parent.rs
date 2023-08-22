use glam::Affine3A;
use hecs::Entity;

/// Component added to indicate that an entity has a parent
/// Used by `update_global_transform_with_parent_system`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parent {
    /// The parent entity
    pub entity: Entity,
    /// The transform that takes coordinates from the coordinate system of the child to the parent
    pub from_child: Affine3A,
}
