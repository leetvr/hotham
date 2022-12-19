use glam::{Quat, Vec3};
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
    pub direction: Vec3,
    /// The range of the light. -1 indicates infinite range.
    pub falloff: f32,

    /// RGB value for the color of the light in linear space.
    pub color: Vec3,
    /// Brightness of light in the type specific units.
    /// Point and spot lights use luminous intensity in candela (lm/sr), while directional lights use
    /// illuminance in lux (lm/m2)
    pub intensity: f32,

    /// The position of the light
    pub position: Vec3,

    /// Pre-computed value for spotlights
    pub light_angle_scale: f32,

    /// Pre-computed value for spotlights
    pub light_angle_offset: f32,
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
        direction: Vec3,
        range: f32,
        intensity: f32,
        color: Vec3,
        position: Vec3,
        inner_cone_angle: f32,
        outer_cone_angle: f32,
    ) -> Self {
        let (scale, offset) = get_offet_and_scale(inner_cone_angle, outer_cone_angle);

        let falloff = get_falloff(range);

        Self {
            direction,
            falloff,
            color,
            intensity,
            position,
            light_angle_scale: scale,
            light_angle_offset: offset,
            light_type: LIGHT_TYPE_SPOT,
        }
    }

    /// Create a new directional light
    pub fn new_directional(direction: Vec3, intensity: f32, color: Vec3) -> Self {
        Self {
            direction,
            color,
            intensity,
            light_type: LIGHT_TYPE_DIRECTIONAL,
            ..Default::default()
        }
    }

    /// Create a new point light
    pub fn new_point(position: Vec3, range: f32, intensity: f32, color: Vec3) -> Self {
        Self {
            position,
            falloff: get_falloff(range),
            color,
            intensity,
            light_type: LIGHT_TYPE_POINT,
            ..Default::default()
        }
    }

    pub(crate) fn from_gltf(light: &gltf::khr_lights_punctual::Light, node: &gltf::Node) -> Self {
        // TODO: Technically scale could apply here as well.
        let (translation, rotation, _) = node.transform().decomposed();
        let rotation = Quat::from_array(rotation);
        let intensity = light.intensity();
        let color = light.color().into();
        let range = light.range().unwrap_or(-1.);
        let direction = rotation * Vec3::NEG_Z;
        let position = translation.into();

        match light.kind() {
            gltf::khr_lights_punctual::Kind::Directional => {
                Light::new_directional(direction, intensity, color)
            }
            gltf::khr_lights_punctual::Kind::Point => {
                Light::new_point(position, range, intensity, color)
            }
            gltf::khr_lights_punctual::Kind::Spot {
                inner_cone_angle,
                outer_cone_angle,
            } => Light::new_spotlight(
                direction,
                range,
                intensity,
                color,
                position,
                inner_cone_angle,
                outer_cone_angle,
            ),
        }
    }
}

fn get_offet_and_scale(inner_cone_angle: f32, outer_cone_angle: f32) -> (f32, f32) {
    let inner_cone_angle = inner_cone_angle
        .abs()
        .clamp(0.5_f32.to_radians(), std::f32::consts::FRAC_PI_2);
    let outer_cone_angle = outer_cone_angle
        .abs()
        .clamp(0.5_f32.to_radians(), std::f32::consts::FRAC_PI_2);
    let inner_cone_angle = inner_cone_angle.min(outer_cone_angle);
    let cos_inner = inner_cone_angle.cos();
    let cos_outer = outer_cone_angle.cos();
    let scale = 1.0 / f32::max(0.001, cos_inner - cos_outer);
    let offset = -cos_outer * scale;
    (scale, offset)
}

// TODO: is this correct? Filament's glTF importer just does this
fn get_falloff(range: f32) -> f32 {
    let squared = range * range;
    if squared > 0.0 {
        1. / squared
    } else {
        0.0
    }
}
