use nalgebra::Matrix4;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformMatrix(pub Matrix4<f32>);

impl Default for TransformMatrix {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}
