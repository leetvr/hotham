use cgmath::{Matrix4, Quaternion, SquareMatrix, Vector3, Zero};
use legion::Entity;

#[derive(Clone, PartialEq, Debug)]
pub struct Transform {
    pub translation: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub scale: Vector3<f32>,
    pub global_matrix: Matrix4<f32>,
    pub local_matrix: Matrix4<f32>,
    pub parent: Option<Entity>,
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            translation: Vector3::zero(),
            rotation: Quaternion::zero(),
            scale: Vector3::new(1.0, 1.0, 1.0),
            global_matrix: Matrix4::identity(),
            local_matrix: Matrix4::identity(),
            parent: None,
        }
    }
}
