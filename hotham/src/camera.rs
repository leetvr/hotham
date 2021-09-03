use anyhow::{anyhow, Result};
use nalgebra::{Isometry3, Matrix4, Translation3, UnitQuaternion, Vector3};
use openxr::View;

use crate::util::posef_to_isometry;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Isometry3<f32>,
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
    pub fn update_view_matrix(&mut self, view: &View) -> Result<Matrix4<f32>> {
        // Convert values from OpenXR format
        let camera_position = posef_to_isometry(view.pose);
        self.position = camera_position;

        self.view_matrix = self.build_matrix()?;
        Ok(self.view_matrix)
    }

    pub fn build_matrix(&self) -> Result<Matrix4<f32>> {
        self.position
            .to_homogeneous()
            .try_inverse()
            .ok_or_else(|| anyhow!("Unable to invert view Matrix!"))
    }
}
