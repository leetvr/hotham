#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnimationController {
    pub blend_from: usize,
    pub blend_to: usize,
    pub blend_amount: f32,
}
