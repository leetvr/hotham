use hecs::Entity;
use nalgebra::{UnitQuaternion, Vector3};

/// A component that allows an entity to be animated.
/// Usually added by `gltf_loader` if the node contains animation data.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    /// The entity that is affected by this animation
    pub target: Entity,
    /// Rotations for this animation
    pub rotations: Vec<UnitQuaternion<f32>>,
    /// Scales for this animation
    pub scales: Vec<Vector3<f32>>,
    /// Translations for this animation
    pub translations: Vec<Vector3<f32>>,
}
