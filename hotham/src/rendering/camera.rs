use nalgebra::{vector, Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3, Vector4};
use openxr::View;

use crate::util::posef_to_isometry;

#[derive(Debug, Clone)]
/// The Camera, or View, in a scene.
pub struct Camera {
    /// The camera's position in space
    pub position: Isometry3<f32>,
    /// The view matrix
    pub view_matrix: Matrix4<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        let t = Translation3::from(Vector3::zeros());
        Self {
            position: Isometry3::from_parts(t, UnitQuaternion::identity()),
            view_matrix: Matrix4::identity(),
        }
    }
}

impl Camera {
    /// Update the camera's position from an OpenXR view
    pub fn update(&mut self, view: &View) -> Matrix4<f32> {
        // Convert values from OpenXR format
        let camera_position = posef_to_isometry(view.pose);
        self.position = camera_position;

        self.view_matrix = self.build_matrix();
        self.view_matrix
    }

    /// Get the camera's position
    pub fn position(&self) -> Vector4<f32> {
        let p = self.position.translation.vector;
        vector![p[0], p[1], p[2], 1.]
    }

    /// Build the camera's view matrix
    pub fn build_matrix(&self) -> Matrix4<f32> {
        self.position.inverse().to_homogeneous()
    }
}
