use super::hand::Handedness;

/// A component added to an entity to allow users to interact with `UIPanels` using their
/// controllers.
pub struct Pointer {
    /// Which hand is the pointer in?
    pub handedness: Handedness,
    /// How much has the trigger been pulled down?
    pub trigger_value: f32,
}
