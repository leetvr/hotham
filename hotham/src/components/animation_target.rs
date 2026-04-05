use glam::{Quat, Vec3};
use hecs::Entity;

/// A component that allows an entity to be animated.
/// Usually added by `gltf_loader` if the node contains animation data.
#[derive(Debug, Clone, PartialEq)]
pub struct AnimationTarget {
    /// The entity that is affected by this animation
    pub target: Entity,
    /// The outputs used to interpolate
    pub outputs: AnimationOutputs,
    /// The times used to interpolate
    pub times: Vec<f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnimationOutputs {
    /// Scales for this animation
    Scales(Vec<Vec3>),
    /// Translations for this animation
    Translation(Vec<Vec3>),
    /// Rotations for this animation
    Rotation(Vec<Quat>),
}
