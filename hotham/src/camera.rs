use cgmath::*;
use openxr::View;

#[derive(Debug, Clone)]
pub struct Camera {
    pub position: Point3<f32>,
    pub pitch: Rad<f32>,
    pub roll: Rad<f32>,
    pub yaw: Rad<f32>,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            position: Point3::origin(),
            pitch: Rad(0.0),
            roll: Rad(0.0),
            yaw: Rad(0.0),
        }
    }
}

impl Camera {
    pub fn update_view_matrix(&mut self, views: &Vec<View>, _delta_time: f32) -> Matrix4<f32> {
        // Convert values from OpenXR format
        let orientation = views[0].pose.orientation;
        let camera_rotation = Euler::from(Quaternion::new(
            orientation.x,
            orientation.y,
            orientation.z,
            orientation.w,
        ));

        self.pitch = camera_rotation.x;
        self.roll = camera_rotation.y;
        self.yaw = camera_rotation.z;

        let position = views[0].pose.position;
        self.position = Point3::new(position.x, position.y, position.z);

        // let camera_x = 0.0;
        // let camera_y = 1.8;
        // let camera_z = 0.1 + 0.001 * delta_time;

        let camera_center = Point3::new(0.0, 0.0, 0.0);
        let camera_up = vec3(0.0, 1.0, 0.0);
        let _direction = vec3(self.yaw.0.cos(), self.pitch.0.sin(), self.roll.0.cos());

        // Matrix4::look_to_rh(self.position, direction, camera_up);
        Matrix4::look_at_rh(self.position, camera_center, camera_up)
    }
}
