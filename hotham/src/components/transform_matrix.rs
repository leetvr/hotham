use nalgebra::Matrix4;

/// Component used to represent the world-space transform of the entity in the renderer.
/// This is the transformation from local to world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransformMatrix(pub Matrix4<f32>);

impl Default for TransformMatrix {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}
