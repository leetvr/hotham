#[derive(Debug, Clone)]
pub struct Impulse {
    pub value: glam::Vec3,
}

impl Impulse {
    pub fn new(value: glam::Vec3) -> Self {
        Self { value }
    }
}
