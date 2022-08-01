use nalgebra::Matrix4;

/// Component used to represent the global transform of the entity in the renderer.
/// This is the transformation from local to global space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform(pub Matrix4<f32>);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}
