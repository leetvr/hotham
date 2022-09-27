use gltf::scene::Transform as TransformData;
use nalgebra::{vector, Isometry3, Quaternion, Unit, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};

/// The component's position in global space (ie. the game simulation), relative to its parent.
///
/// There are two ways to set an entity's position in Hotham:
///
/// 1. **Game controlled** - this entity will have its rigid body position set by the **game** simulation
/// 1. **Physics controlled** - this entity will have its position set by the **physics** simulation
///
/// If an entity has the [`super::PhysicsControlled`] component and a [`super::RigidBody`], then you
/// are indicating that you want this entity's position in the game simulation (ie. its global position)
/// to be entirely controlled by the physics simulation.
///
/// Otherwise, you can just modify [`LocalTransform`] and your entity will have its position in the game
/// simulation AND the physics simulation (if it has a [`super::RigidBody`]) set relative to its [`super::Parent`].
///
/// If the entity doesn't have a [`super::Parent`], then the global transform is just whatever you've set here.
#[derive(Clone, PartialEq, Debug, Copy, Deserialize, Serialize)]
pub struct LocalTransform {
    /// The translation of the entity
    pub translation: Vector3<f32>,
    /// The rotation of the entity
    pub rotation: UnitQuaternion<f32>,
    /// The non-uniform scale of the entity
    pub scale: Vector3<f32>,
}

impl Default for LocalTransform {
    fn default() -> Self {
        Self {
            translation: Vector3::zeros(),
            rotation: UnitQuaternion::identity(),
            scale: vector![1.0, 1.0, 1.0],
        }
    }
}

impl LocalTransform {
    pub(crate) fn load(transform_data: TransformData) -> LocalTransform {
        let (t, r, s) = transform_data.decomposed();
        let translation = vector![t[0], t[1], t[2]];
        let rotation = Unit::new_normalize(Quaternion::new(r[3], r[0], r[1], r[2]));
        let scale = vector![s[0], s[1], s[2]];

        LocalTransform {
            scale,
            rotation,
            translation,
        }
    }

    /// Convenience function to convert the `LocalTransform` into a `nalgebra::Isometry3`
    pub fn position(&self) -> Isometry3<f32> {
        Isometry3 {
            rotation: self.rotation,
            translation: self.translation.into(),
        }
    }

    /// Update the translation and rotation from a [`nalgebra::Isometry3`]
    pub fn update_from_isometry(&mut self, isometry: &Isometry3<f32>) {
        self.translation = isometry.translation.vector;
        self.rotation = isometry.rotation;
    }
}
