use cgmath::{Matrix4, SquareMatrix};

#[derive(Debug, Clone)]
pub struct TransformMatrix(pub Matrix4<f32>);

impl Default for TransformMatrix {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}
