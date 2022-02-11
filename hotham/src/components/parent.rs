use hecs::Entity;

/// Component added to indicate that an entity has a parent
/// Used by `update_parent_transform_matrix_system`
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parent(pub Entity);
