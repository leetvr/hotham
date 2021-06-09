use cgmath::{Matrix4, SquareMatrix};

#[derive(Clone, Debug)]
pub struct ViewMatrix {
    pub model: Matrix4<f32>,
    pub view: Matrix4<f32>,
    pub projection: Matrix4<f32>,
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
