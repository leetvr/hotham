use gltf::Material as MaterialData;

use crate::{
    asset_importer::ImportContext,
    rendering::texture::{Texture, TextureUsage, NO_TEXTURE},
};

use bitflags::bitflags;

bitflags! {
        /// Flags used by the shader to do shit
    pub struct MaterialFlags: u32 {
        /// Do we have base color texture?
        const HAS_BASE_COLOR_TEXTURE = 1 << 0;
        /// Do we have metallic roughness texture?
        const HAS_METALLIC_ROUGHNESS_TEXTURE = 1 << 1;
        /// Do we have a normal map?
        const HAS_NORMAL_MAP = 1 << 2;
        /// Do we have an AO texture?
        const HAS_AO_TEXTURE = 1 << 3;
        /// Do we have an emission texture?
        const HAS_EMISSION_TEXTURE = 1 << 4;
        /// Are we using unlit workflow?
        const UNLIT_WORKFLOW = 1 << 5;
    }
}

/// Material index into the default material
pub static NO_MATERIAL: usize = 0;

/// Mostly maps to the [glTF material spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#materials) and
/// added by default by the `gltf_loader`
#[repr(C)]
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The flags and base_texture_id are stored as two u16 packed into a single u32. The flags are stored in the least significant bits.
    pub packed_flags_and_base_texture_id: u32,
    /// The base color of the material
    pub packed_base_color_factor: u32,
    /// The metallic amd roughness factors, followed by alpha mask cutoff packed with packUnorm4x8.
    pub packed_metallic_roughness_factor_alpha_mask_cutoff: u32,
}

impl Default for Material {
    fn default() -> Self {
        Material::gltf_default()
    }
}

impl Material {
    /// Load a material from a glTF document
    pub(crate) fn load(material: MaterialData, import_context: &mut ImportContext) {
        let pbr_metallic_roughness = material.pbr_metallic_roughness();

        // Base Color
        let base_color_texture_info = pbr_metallic_roughness.base_color_texture();
        let base_color_texture_set = base_color_texture_info
            .map(|i| Texture::load(i.texture(), TextureUsage::BaseColor, import_context))
            .unwrap_or(NO_TEXTURE);

        // Metallic Roughness
        let metallic_roughness_texture_info = pbr_metallic_roughness.metallic_roughness_texture();
        let metallic_roughness_texture_set = metallic_roughness_texture_info
            .map(|i| {
                Texture::load(
                    i.texture(),
                    TextureUsage::MetallicRoughnessOcclusion,
                    import_context,
                )
            })
            .unwrap_or(NO_TEXTURE);

        // Normal map
        let normal_texture_info = material.normal_texture();
        let normal_texture_set = normal_texture_info
            .map(|i| Texture::load(i.texture(), TextureUsage::Normal, import_context))
            .unwrap_or(NO_TEXTURE);

        // For performance, we don't allow unpacked AO textures.
        //
        // see: https://github.com/leetvr/hotham/issues/395
        let has_occlusion_texture = if let Some(occlusion_texture_info) =
            material.occlusion_texture()
        {
            // This is.. quite ugly.
            if Some(occlusion_texture_info.texture().source().index())
                == material
                    .pbr_metallic_roughness()
                    .metallic_roughness_texture()
                    .map(|t| t.texture().source().index())
            {
                true
            } else {
                // Attempting to use unpacked AO texture. Warn the developer.
                println!("[HOTHAM_TEXTURE] WARNING: It looks like you're storing occlusion in a separate image! This is not supported by Hotham and will be ignored.");
                false
            }
        } else {
            false
        };

        // Emission
        let emissive_texture_info = material.emissive_texture();
        let emissive_texture_set = emissive_texture_info
            .map(|i| Texture::load(i.texture(), TextureUsage::Emission, import_context))
            .unwrap_or(NO_TEXTURE);

        let mut material_flags = MaterialFlags::empty();
        if base_color_texture_set != NO_TEXTURE {
            material_flags.insert(MaterialFlags::HAS_BASE_COLOR_TEXTURE);
        }

        if metallic_roughness_texture_set != NO_TEXTURE {
            material_flags.insert(MaterialFlags::HAS_METALLIC_ROUGHNESS_TEXTURE);
        }

        if normal_texture_set != NO_TEXTURE {
            material_flags.insert(MaterialFlags::HAS_NORMAL_MAP);
        }

        if emissive_texture_set != NO_TEXTURE {
            material_flags.insert(MaterialFlags::HAS_EMISSION_TEXTURE);
        }

        if has_occlusion_texture {
            material_flags.insert(MaterialFlags::HAS_AO_TEXTURE);
        }

        if material.unlit() {
            material_flags.insert(MaterialFlags::UNLIT_WORKFLOW);
        }

        // Don't allow non-sensical flags
        assert_ne!(material_flags, MaterialFlags::HAS_EMISSION_TEXTURE);
        assert_ne!(material_flags, MaterialFlags::HAS_AO_TEXTURE);
        assert_ne!(
            material_flags,
            MaterialFlags::HAS_AO_TEXTURE | MaterialFlags::HAS_EMISSION_TEXTURE
        );

        // Collect the material properties.
        let material = Material {
            packed_flags_and_base_texture_id: pack2x16(material_flags.bits, base_color_texture_set),
            packed_base_color_factor: pack_unorm4x8(&pbr_metallic_roughness.base_color_factor()),
            packed_metallic_roughness_factor_alpha_mask_cutoff: pack_unorm4x8(&[
                pbr_metallic_roughness.metallic_factor(),
                pbr_metallic_roughness.roughness_factor(),
                material.alpha_cutoff().unwrap_or(0.0),
                0.0,
            ]),
        };

        // Then push it into the materials buffer
        unsafe {
            import_context
                .render_context
                .resources
                .materials_buffer
                .push(&material);
        }
    }

    /// Create a simple, unlit, white coloured material.
    pub fn unlit_white() -> Material {
        Material {
            packed_flags_and_base_texture_id: MaterialFlags::UNLIT_WORKFLOW.bits,
            packed_base_color_factor: u32::MAX,
            packed_metallic_roughness_factor_alpha_mask_cutoff: pack_unorm4x8(&[
                1.0, 1.0, 0.0, 0.0,
            ]),
        }
    }

    /// The default material, reasonably close to what's defined by the glTF 2.0 spec
    /// https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-material-pbrmetallicroughness
    pub fn gltf_default() -> Self {
        Self {
            packed_flags_and_base_texture_id: MaterialFlags::empty().bits,
            packed_base_color_factor: u32::MAX,
            packed_metallic_roughness_factor_alpha_mask_cutoff: pack_unorm4x8(&[
                1.0, 1.0, 0.0, 0.0,
            ]),
        }
    }
}

/// Convert normalized floating-point values into 8-bit integer values and pack them into an u32.
/// First value is stored in least significant bits. This works the same as packUnorm4x8 in GLSL.
pub fn pack_unorm4x8(ary: &[f32; 4]) -> u32 {
    let mut packed: u32 = 0;
    for value in ary.iter().rev() {
        let packed_value = (value.clamp(0.0, 1.0) * 255.0).round() as u32;
        packed = (packed << 8) | packed_value;
    }
    packed
}

/// Pack the least significant 16 bits from two u32 into a single u32.
pub fn pack2x16(lsb: u32, msb: u32) -> u32 {
    (msb << 16) | (lsb & 0xFFFF)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_unorm4x8_test() {
        assert_eq!(pack_unorm4x8(&[1.0, 0.0, 0.0, 0.0]), 0x000000FF);
        assert_eq!(pack_unorm4x8(&[0.0, 1.0, 0.0, 0.0]), 0x0000FF00);
        assert_eq!(pack_unorm4x8(&[0.0, 0.0, 1.0, 0.0]), 0x00FF0000);
        assert_eq!(pack_unorm4x8(&[0.0, 0.0, 0.0, 1.0]), 0xFF000000);
    }
}
