use cgmath::Matrix4;
use legion::Entity;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Joint {
    pub skeleton_root: Entity,
    pub inverse_bind_matrix: Matrix4<f32>,
}
