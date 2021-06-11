use cgmath::{Matrix4, SquareMatrix};

#[derive(Clone, Debug)]
pub struct ViewMatrix {
    pub view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
    pub delta_time: f32,
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self {
            view: Matrix4::identity(),
            projection: Matrix4::identity(),
            delta_time: 0.0,
        }
    }
}
