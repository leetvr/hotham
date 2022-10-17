use glam::{Affine3A, Quat, Vec3};
use gltf::scene::Transform as TransformData;
use serde::{Deserialize, Serialize};

use crate::util::{decompose_isometry, isometry_from_affine};

/// The component's position in global space (ie. the game simulation), relative to its parent.
///
/// There are two ways to set an entity's position in Hotham:
///
/// 1. **Game controlled** - this entity will have its rigid body position set by the **game** simulation
/// 1. **Physics controlled** - this entity will have its position set by the **physics** simulation
///
/// If an entity has a [`super::RigidBody`] component with a `body_type` of [`super::physics::BodyType::Dynamic`], then you
/// are indicating that you want this entity's position in the game simulation (ie. its global position)
/// to be entirely controlled by the physics simulation.
///
/// Otherwise, you can just modify [`LocalTransform`] and your entity will have its position in the game
/// simulation AND the physics simulation (if it has a [`super::RigidBody`] and/or [`super::Collider`]) set relative to its [`super::Parent`].
///
/// If the entity doesn't have a [`super::Parent`], then the [`super::GlobalTransform`] is just whatever you've set here.
#[derive(Clone, PartialEq, Debug, Copy, Deserialize, Serialize)]
pub struct LocalTransform {
    /// The translation of the entity
    pub translation: Vec3,
    /// The rotation of the entity
    pub rotation: Quat,
    /// The non-uniform scale of the entity
    pub scale: Vec3,
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl LocalTransform {
    pub(crate) fn load(transform_data: TransformData) -> LocalTransform {
        let (t, r, s) = transform_data.decomposed();
        let translation = t.into();
        let rotation = Quat::from_xyzw(r[0], r[1], r[2], r[3]);
        let scale = s.into();

        LocalTransform {
            scale,
            rotation,
            translation,
        }
    }

    pub fn from_rotation_translation(rotation: Quat, translation: Vec3) -> Self {
        LocalTransform {
            rotation,
            translation,
            ..Default::default()
        }
    }

    /// Convenience function to convert the [`LocalTransform`] into a [`rapier3d::na::Isometry3`]
    pub fn to_isometry(&self) -> rapier3d::na::Isometry3<f32> {
        isometry_from_affine(&self.to_affine())
    }

    /// Update the translation and rotation from a [`rapier3d::na::Isometry3`]
    pub fn update_from_isometry(&mut self, isometry: &rapier3d::na::Isometry3<f32>) {
        (self.rotation, self.translation) = decompose_isometry(isometry);
    }

    /// Update the scale, rotation and rotation from a [`glam::Affine3A`]
    pub fn update_from_affine(&mut self, transform: &glam::Affine3A) {
        (self.scale, self.rotation, self.translation) = transform.to_scale_rotation_translation();
    }

    /// Update ONLY rotation and rotation from a [`glam::Affine3A`]
    pub fn update_rotation_translation_from_affine(&mut self, transform: &glam::Affine3A) {
        (_, self.rotation, self.translation) = transform.to_scale_rotation_translation();
    }

    /// Convenience function to convert the [`LocalTransform`] into a [`glam::Affine3A`]
    pub fn to_affine(&self) -> Affine3A {
        Affine3A::from_scale_rotation_translation(self.scale, self.rotation, self.translation)
    }
}

impl From<LocalTransform> for Affine3A {
    fn from(l: LocalTransform) -> Self {
        l.to_affine()
    }
}
