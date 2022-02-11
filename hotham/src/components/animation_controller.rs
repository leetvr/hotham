#[derive(Debug, Clone, PartialEq, Default)]

/// Component that controls how an `AnimationTarget` should be animated.
/// Added by `gltf_loader` to the root node if its children contain animation data.
pub struct AnimationController {
    /// The amount to blend from
    pub blend_from: usize,
    /// The amount to blend to
    pub blend_to: usize,
    /// The total blend amount
    pub blend_amount: f32,
}
