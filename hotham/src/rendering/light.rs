use nalgebra::Vector3;
use serde::{Deserialize, Serialize};

/// A directional light.
pub const LIGHT_TYPE_DIRECTIONAL: u32 = 0;
/// A point light.
pub const LIGHT_TYPE_POINT: u32 = 1;
/// A spot light.
pub const LIGHT_TYPE_SPOT: u32 = 2;
/// No light
pub const LIGHT_TYPE_NONE: u32 = u32::MAX;
/// Maximum number of dynamic lights in a scene
pub const MAX_LIGHTS: usize = 4;

/// Representation of a light in a scene, based on the KHR_lights_punctual extension:
/// https://github.com/KhronosGroup/glTF/tree/master/extensions/2.0/Khronos/KHR_lights_punctual
#[derive(Deserialize, Serialize, Clone, Debug, Copy, Default)]
#[repr(C, align(16))]
pub struct Light {
    /// The direction the light is facing.
    pub direction: Vector3<f32>,
    /// The range of the light. -1 indicates infinite range.
    pub range: f32,

    /// RGB value for the color of the light in linear space.
    pub color: Vector3<f32>,
    /// Brightness of light in the type specific units.
    /// Point and spot lights use luminous intensity in candela (lm/sr), while directional lights use
    /// illuminance in lux (lm/m2)
    pub intensity: f32,

    /// The position of the light
    pub position: Vector3<f32>,
    /// Cosine of the angle, in radians, from centre of spotlight where falloff begins.
    pub inner_cone_cos: f32,

    /// Cosine of the angle, in radians, from centre of spotlight where falloff ends.
    pub outer_cone_cos: f32,
    /// The type of the light. LIGHT_TYPE_NONE indicates to the fragment shader that this light is empty.
    pub light_type: u32,
}

impl Light {
    /// Create a "NONE" light. Indicates to the fragment shader that it should not do anything with this light.
    pub fn none() -> Self {
        Self {
            light_type: LIGHT_TYPE_NONE,
            ..Default::default()
        }
    }

    /// Create a new spotlight
    pub fn new_spotlight(
        direction: Vector3<f32>,
        range: f32,
        intensity: f32,
        position: Vector3<f32>,
        inner_cone_angle: f32,
        outer_cone_angle: f32,
    ) -> Self {
        Self {
            direction,
            range,
            color: [1., 1., 1.].into(),
            intensity,
            position,
            inner_cone_cos: inner_cone_angle.cos(),
            outer_cone_cos: outer_cone_angle.cos(),
            light_type: LIGHT_TYPE_SPOT,
        }
    }
}
