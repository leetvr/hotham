use anyhow::{anyhow, Result};
use cgmath::*;
use openxr::View;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Vector3<f32>,
    pub orientation: Quaternion<f32>,
    pub view_matrix: Matrix4<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: vec3(0.0, 0.0, 0.0),
            orientation: Quaternion::zero(),
            view_matrix: Matrix4::identity(),
        }
    }
}

impl Camera {
    pub fn update_view_matrix(&mut self, view: &View, _delta_time: f32) -> Result<Matrix4<f32>> {
        // Convert values from OpenXR format
        let (camera_position, camera_orientation) = convert_view(view);
        self.position = camera_position;
        self.orientation = camera_orientation;

        self.view_matrix = self.build_matrix()?;
        Ok(self.view_matrix)
    }

    pub fn build_matrix(&self) -> Result<Matrix4<f32>> {
        let scale = Matrix4::from_scale(1.0);
        let euler = Euler::from(self.orientation);
        let rotation_x = Matrix4::from_angle_x(euler.x);
        let rotation_y = Matrix4::from_angle_y(-euler.y);
        let rotation_z = Matrix4::from_angle_z(-euler.z);
        let rotation = rotation_x * rotation_y * rotation_z;

        let translation = Matrix4::from_translation(self.position);

        let matrix = translation * rotation * scale;
        let matrix = matrix
            .inverse_transform()
            .ok_or(anyhow!("Unable to invert matrix"))?;
        let numbers: &[f32; 16] = matrix.as_ref();
        for n in numbers {
            if n.is_nan() {
                return Err(anyhow!("View matrix is broken: {:?}", self));
            }
        }

        Ok(matrix)
    }
}

pub fn convert_view(view: &View) -> (Vector3<f32>, Quaternion<f32>) {
    let orientation: mint::Quaternion<f32> = view.pose.orientation.into();
    let orientation = Quaternion::from(orientation);

    let position: mint::Vector3<f32> = view.pose.position.into();
    let position = Vector3::from(position);

    return (position, orientation);
}

fn _working_matrix() -> Matrix4<f32> {
    let up = vec3(0.0, 1.0, 0.0);
    let camera_position = point3(0.0, 0.0, 1.0);
    let camera_center = point3(0.0, 0.0, 0.0);
    let direction = camera_center - camera_position;

    Matrix4::look_to_rh(camera_position, direction, up)
}
