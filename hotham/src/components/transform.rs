use gltf::scene::Transform as TransformData;
use nalgebra::{vector, Isometry3, Quaternion, Unit, UnitQuaternion, Vector3};
use serde::{Deserialize, Serialize};

/// Component that represents the transform of the entity in world space
/// Added automatically by `gltf_loader`, will be overriden if the entity also contains a `rigid_body` and
/// `update_rigid_body_transforms_system` is running
#[derive(Clone, PartialEq, Debug, Copy, Deserialize, Serialize)]
pub struct Transform {
    /// The translation of the entity
    pub translation: Vector3<f32>,
    /// The rotation of the entity
    pub rotation: UnitQuaternion<f32>,
    /// The non-uniform scale of the entity
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::zeros(),
            rotation: UnitQuaternion::identity(),
            scale: vector![1.0, 1.0, 1.0],
        }
    }
}

impl Transform {
    pub(crate) fn load(transform_data: TransformData) -> Transform {
        let (t, r, s) = transform_data.decomposed();
        let translation = vector![t[0], t[1], t[2]];
        let rotation = Unit::new_normalize(Quaternion::new(r[3], r[0], r[1], r[2]));
        let scale = vector![s[0], s[1], s[2]];

        Transform {
            scale,
            rotation,
            translation,
        }
    }

    /// Convenience function to convert the `Transform` into a `nalgebra::Isometry3`
    pub fn position(&self) -> Isometry3<f32> {
        Isometry3 {
            rotation: self.rotation,
            translation: self.translation.into(),
        }
    }
}
