use glam::{Quat, Vec3};
use hecs::Entity;

/// A component that allows an entity to be animated.
/// Usually added by `gltf_loader` if the node contains animation data.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    /// The entity that is affected by this animation
    pub target: Entity,
    /// Rotations for this animation
    pub rotations: Vec<Quat>,
    /// Scales for this animation
    pub scales: Vec<Vec3>,
    /// Translations for this animation
    pub translations: Vec<Vec3>,
}
