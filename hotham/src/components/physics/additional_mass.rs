#[derive(Debug, Clone, Default)]
pub struct AdditionalMass {
    pub value: f32,
}

impl AdditionalMass {
    pub fn new(value: f32) -> Self {
        Self { value }
    }
}
