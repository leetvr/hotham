use anyhow::{anyhow, Result};
use cgmath::*;
use openxr::View;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Point3<f32>,
    pub pitch: Rad<f32>,
    pub roll: Rad<f32>,
    pub yaw: Rad<f32>,
    pub view_matrix: Matrix4<f32>,
    pub direction: Vector3<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0, 1.0).into(),
            pitch: Deg(0.0).into(),
            roll: Deg(0.0).into(),
            yaw: Deg(0.0).into(),
            view_matrix: Matrix4::identity(),
            direction: vec3(0.0, 0.0, 0.0),
        }
    }
}

impl Camera {
    pub fn update_view_matrix(
        &mut self,
        views: &Vec<View>,
        _delta_time: f32,
    ) -> Result<Matrix4<f32>> {
        // Convert values from OpenXR format
        let (camera_position, camera_rotation) = convert_view(&views[0]);
        self.position = camera_position;
        self.yaw = camera_rotation.y;
        self.pitch = camera_rotation.x;

        self.view_matrix = self.build_matrix();

        self.sanity_check()?;
        Ok(self.view_matrix)
    }

    pub fn build_matrix(&self) -> Matrix4<f32> {
        let rotation = Euler::new(self.pitch * -1.0, self.yaw * -1.0, self.roll * -1.0);
        let rotation = Matrix4::from(Matrix3::from(rotation));

        let translation = Matrix4::from_translation(self.position.to_vec() * -1.0);

        rotation * translation
    }

    pub fn sanity_check(&self) -> Result<()> {
        let numbers: &[f32; 16] = self.view_matrix.as_ref();
        for n in numbers {
            if n.is_nan() {
                return Err(anyhow!("View matrix is broken: {:?}", self));
            }
        }

        Ok(())
    }
}

fn convert_view(view: &View) -> (Point3<f32>, Euler<Rad<f32>>) {
    let orientation = view.pose.orientation;
    let orientation = Quaternion::new(orientation.w, orientation.x, orientation.y, orientation.z);
    let rotation = Euler::from(orientation);

    let position = view.pose.position;
    let position = Point3::new(position.x, position.y, position.z);

    return (position, rotation);
}

fn _working_matrix() -> Matrix4<f32> {
    let up = vec3(0.0, 1.0, 0.0);
    let camera_position = point3(0.0, 0.0, 1.0);
    let camera_center = point3(0.0, 0.0, 0.0);
    let direction = camera_center - camera_position;

    Matrix4::look_to_rh(camera_position, direction, up)
}
