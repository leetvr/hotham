use hecs::Entity;

/// Component added to indicate that an entity has a parent
/// Used by `update_global_transform_with_parent_system`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parent(pub Entity);
