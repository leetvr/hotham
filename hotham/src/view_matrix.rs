use cgmath::{Matrix4, SquareMatrix};

#[derive(Clone, Debug)]
pub struct ViewMatrix {
    model: Matrix4<f32>,
    view: Matrix4<f32>,
    projection: Matrix4<f32>,
}

impl Default for ViewMatrix {
    fn default() -> Self {
        Self {
            model: Matrix4::identity(),
            view: Matrix4::identity(),
            projection: Matrix4::identity(),
        }
    }
}
