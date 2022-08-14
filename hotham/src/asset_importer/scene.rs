use crate::rendering::light::Light;

use super::Models;

/// Representation of a glTF Scene
pub struct Scene {
    /// The models in the scene
    pub models: Models,
    /// The lights in the scene
    pub lights: Vec<Light>,
}
