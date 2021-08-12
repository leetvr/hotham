use cgmath::{Quaternion, Vector3, Zero};
use gltf::scene::Transform as TransformData;

#[derive(Clone, PartialEq, Debug)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::zero(),
            rotation: Quaternion::zero(),
            scale: Vector3::new(1.0, 1.0, 1.0),
        }
    }
}

impl Transform {
    pub fn load(transform_data: TransformData) -> Transform {
        let (t, r, s) = transform_data.decomposed();
        let translation = Vector3::new(t[0], t[1], t[2]);
        let rotation = Quaternion::new(r[3], r[0], r[1], r[2]);
        let scale = Vector3::new(s[0], s[1], s[2]);

        Transform {
            translation,
            rotation,
            scale,
        }
    }
}
