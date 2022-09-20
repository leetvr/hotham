use nalgebra::{vector, Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3, Vector4};
use openxr::View;

use crate::util::posef_to_isometry;

#[derive(Debug, Clone)]
/// The Camera, or View, in a scene.
pub struct Camera {
    /// The camera's pose in globally oriented stage space
    pub gos_from_view: Isometry3<f32>,
    /// The view matrix
    pub view_from_gos: Matrix4<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        let t = Translation3::from(Vector3::zeros());
        Self {
            gos_from_view: Isometry3::from_parts(t, UnitQuaternion::identity()),
            view_from_gos: Matrix4::identity(),
        }
    }
}

impl Camera {
    /// Update the camera's position from an OpenXR view
    pub fn update(&mut self, view: &View, gos_from_stage: &Isometry3<f32>) -> Matrix4<f32> {
        // Convert values from OpenXR format and use globally oriented stage space instead of stage space
        let stage_from_view = posef_to_isometry(view.pose);
        self.gos_from_view = gos_from_stage * stage_from_view;

        self.view_from_gos = self.build_matrix();
        self.view_from_gos
    }

    /// Get the camera's position
    pub fn position_in_gos(&self) -> Vector4<f32> {
        let p = self.gos_from_view.translation.vector;
        vector![p[0], p[1], p[2], 0.]
    }

    /// Build the camera's view matrix
    pub fn build_matrix(&self) -> Matrix4<f32> {
        self.gos_from_view.inverse().to_homogeneous()
    }
}
