use super::Transform;
use hecs::Entity;

/// A component that allows an entity to be animated.
/// Usually added by `gltf_loader` if the node contains animation data.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    /// The entity that is controlling this animation
    pub controller: Entity,
    /// The transforms that will be applied to this entity
    pub animations: Vec<Vec<Transform>>,
}
