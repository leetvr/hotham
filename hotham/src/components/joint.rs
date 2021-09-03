use legion::Entity;
use nalgebra::Matrix4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    pub skeleton_root: Entity,
    pub inverse_bind_matrix: Matrix4<f32>,
}
