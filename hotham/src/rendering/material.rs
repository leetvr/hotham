use glam::Vec4;
use gltf::Material as MaterialData;

use crate::{
    asset_importer::ImportContext,
    rendering::texture::{Texture, TextureUsage, NO_TEXTURE},
};

/// Tells the fragment shader to use the PBR Metallic Roughness workflow
pub static METALLIC_ROUGHNESS_WORKFLOW: u32 = 0;
/// Tells the fragment shader to use the unlit workflow
pub static UNLIT_WORKFLOW: u32 = 1;

/// Material index into the default material
pub static NO_MATERIAL: usize = 0;

/// Mostly maps to the [glTF material spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#materials) and
/// added by default by the `gltf_loader`
#[repr(C, align(16))]
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// The base color of the material
    pub base_color_factor: Vec4,
    /// What workflow should be used - 0.0 for Metallic Roughness / 1.0 for unlit
    pub workflow: u32,
    /// The base color texture.
    pub base_color_texture_set: u32,
    /// The metallic-roughness texture.
    pub metallic_roughness_texture_id: u32,
    /// Normal texture
    pub normal_texture_set: u32,
    /// Occlusion texture set
    pub occlusion_texture_set: u32,
    /// Emissive texture set
    pub emissive_texture_set: u32,
    /// The factor for the metalness of the material.
    pub metallic_factor: f32,
    /// The factor for the roughness of the material.
    pub roughness_factor: f32,
    /// Alpha mask - see fragment shader
    pub alpha_mask: f32,
    /// Alpha mask cutoff - see fragment shader
    pub alpha_mask_cutoff: f32,
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
        let base_color_factor = Vec4::from(pbr_metallic_roughness.base_color_factor());

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

        // Occlusion

        // This is a little tricky. The common case is that occlusion is packed into the red channel of the metallic roughness texture,
        // but we need to allow for the (unlikely, but still possible) case where it has its own texture.
        //
        // see: https://github.com/leetvr/hotham/issues/395
        let occlusion_texture_set = if let Some(occlusion_texture_info) =
            material.occlusion_texture()
        {
            // This is.. quite ugly.
            if Some(occlusion_texture_info.texture().source().index())
                == material
                    .pbr_metallic_roughness()
                    .metallic_roughness_texture()
                    .map(|t| t.texture().source().index())
            {
                metallic_roughness_texture_set
            } else {
                // This is pretty suboptimal. Warn the developer.
                println!("[HOTHAM_TEXTURE] It looks like you're storing occlusion in a separate image. For best performance, combine it with the MetallicRoughness image");
                Texture::load(
                    occlusion_texture_info.texture(),
                    TextureUsage::MetallicRoughnessOcclusion,
                    import_context,
                )
            }
        } else {
            NO_TEXTURE
        };

        // Emission
        let emissive_texture_info = material.emissive_texture();
        let emissive_texture_set = emissive_texture_info
            .map(|i| Texture::load(i.texture(), TextureUsage::Emission, import_context))
            .unwrap_or(NO_TEXTURE);

        // Factors
        let metallic_factor = pbr_metallic_roughness.metallic_factor();
        let roughness_factor = pbr_metallic_roughness.roughness_factor();

        // Alpha
        let (alpha_mask, alpha_mask_cutoff) = match (material.alpha_mode(), material.alpha_cutoff())
        {
            (gltf::material::AlphaMode::Mask, _) => (1., 0.5),
            (_, Some(alpha_cutoff)) => (1., alpha_cutoff),
            _ => (0., 1.),
        };

        // Workflow
        let workflow = if material.unlit() {
            UNLIT_WORKFLOW
        } else {
            METALLIC_ROUGHNESS_WORKFLOW
        };

        // Collect the material properties.
        let material = Material {
            base_color_factor,
            workflow,
            base_color_texture_set,
            metallic_roughness_texture_id: metallic_roughness_texture_set,
            normal_texture_set,
            occlusion_texture_set,
            emissive_texture_set,
            metallic_factor,
            roughness_factor,
            alpha_mask,
            alpha_mask_cutoff,
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
            workflow: UNLIT_WORKFLOW,
            ..Default::default()
        }
    }

    /// The default material, reasonably close to what's defined by the glTF 2.0 spec
    /// https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#reference-material-pbrmetallicroughness
    pub fn gltf_default() -> Self {
        Self {
            base_color_factor: [1., 1., 1., 1.].into(),
            workflow: METALLIC_ROUGHNESS_WORKFLOW,
            base_color_texture_set: NO_TEXTURE,
            metallic_roughness_texture_id: NO_TEXTURE,
            normal_texture_set: NO_TEXTURE,
            occlusion_texture_set: NO_TEXTURE,
            emissive_texture_set: NO_TEXTURE,
            metallic_factor: 1.0,
            roughness_factor: 1.0,
            alpha_mask: Default::default(),
            alpha_mask_cutoff: Default::default(),
        }
    }
}
