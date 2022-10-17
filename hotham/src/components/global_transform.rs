use crate::util;
use glam::{Affine3A, Quat, Vec3};

use super::LocalTransform;

/// Component used to represent the global transform of the entity in the renderer.
/// This is the transformation from local to global space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform(pub Affine3A);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Affine3A::IDENTITY)
    }
}

impl GlobalTransform {
    /// Convenience function to convert the [`GlobalTransform`] into a [`rapier3d::na::Isometry3`]
    pub fn to_isometry(&self) -> rapier3d::na::Isometry3<f32> {
        util::isometry_from_affine(&self.0)
    }

    /// Convenience function to decompose the [`GlobalTransform`] into its components
    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        self.0.to_scale_rotation_translation()
    }
}

impl From<LocalTransform> for GlobalTransform {
    fn from(l: LocalTransform) -> Self {
        GlobalTransform(l.to_affine())
    }
}
