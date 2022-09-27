use nalgebra::{Isometry3, Matrix4};

/// Component used to represent the global transform of the entity in the renderer.
/// This is the transformation from local to global space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GlobalTransform(pub Matrix4<f32>);

impl Default for GlobalTransform {
    fn default() -> Self {
        Self(Matrix4::identity())
    }
}

impl GlobalTransform {
    /// Get the global transform as an isometry.
    pub fn to_isometry(&self) -> Isometry3<f32> {
        let m = &self.0;
        let translation = m.column(3).xyz();
        let m: nalgebra::Matrix3<f32> = m.fixed_slice::<3, 3>(0, 0).into();
        let rotation = nalgebra::Rotation3::from_matrix(&m);
        Isometry3::from_parts(translation.into(), rotation.into())
    }
}
