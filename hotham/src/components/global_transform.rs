use crate::util;
use glam::{Affine3A, Quat, Vec3};

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
    // Convenience function to create a [`GlobalTransform`]
    #[inline]
    pub fn from_translation(translation: Vec3) -> Self {
        Self(Affine3A::from_translation(translation))
    }

    // Convenience function to create a [`GlobalTransform`]
    #[inline]
    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        Self(Affine3A::from_rotation_translation(rotation, translation))
    }

    // Convenience function to create a [`GlobalTransform`]
    #[inline]
    pub fn from_scale_rotation_translation(scale: Vec3, rotation: Quat, translation: Vec3) -> Self {
        Self(Affine3A::from_scale_rotation_translation(
            scale,
            rotation,
            translation,
        ))
    }

    // Convenience function to create a [`GlobalTransform`]
    #[inline]
    pub fn from_scale(scale: Vec3) -> Self {
        Self(Affine3A::from_scale(scale))
    }

    /// Update the translation and rotation from a [`rapier3d::na::Isometry3`]
    #[inline]
    pub fn update_from_isometry(&mut self, isometry: &rapier3d::na::Isometry3<f32>) {
        let (scale, _, _) = self.to_scale_rotation_translation();
        let (rotation, translation) = util::decompose_isometry(isometry);
        *self = Self::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// Update ONLY rotation and rotation from a [`glam::Affine3A`]
    #[inline]
    pub fn update_rotation_translation_from_affine(&mut self, transform: &glam::Affine3A) {
        let (scale, _, _) = self.to_scale_rotation_translation();
        let (_, rotation, translation) = transform.to_scale_rotation_translation();
        self.0 = Affine3A::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// Convenience function to recreate a [`GlobalTransform`]
    #[inline]
    pub fn update_from_scale_rotation_translation(
        &mut self,
        scale: Vec3,
        rotation: Quat,
        translation: Vec3,
    ) {
        self.0 = Affine3A::from_scale_rotation_translation(scale, rotation, translation);
    }

    /// Convenience function to convert the [`GlobalTransform`] into a [`rapier3d::na::Isometry3`]
    #[inline]
    pub fn to_isometry(&self) -> rapier3d::na::Isometry3<f32> {
        util::isometry_from_affine(&self.0)
    }

    /// Convenience function to decompose the [`GlobalTransform`] into its components
    #[inline]
    pub fn to_scale_rotation_translation(&self) -> (Vec3, Quat, Vec3) {
        self.0.to_scale_rotation_translation()
    }
}
