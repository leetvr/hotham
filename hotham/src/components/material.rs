use anyhow::Result;
use ash::vk;
use gltf::{texture::Info, Material as MaterialData};
use nalgebra::{vector, Vector4};

use crate::{resources::VulkanContext, texture::Texture};

/// A component that instructs the renderer how an entity should look when rendered
/// Mostly maps to the [glTF material spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#materials) and
/// added by default by the `gltf_loader`
#[repr(C)]
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Material {
    /// The base colour of the material
    pub base_colour_factor: Vector4<f32>,
    /// The color and intensity of the light being emitted by the material
    pub emmissive_factor: Vector4<f32>,
    /// How diffuse is this material?
    pub diffuse_factor: Vector4<f32>,
    /// How specular is this material?
    pub specular_factor: Vector4<f32>,
    /// What workflow should be used - 0.0 for Metalic Roughness / 1.0 for Specular Glossiness / 2.0 for unlit
    pub workflow: f32,
    /// The base color texture.
    pub base_color_texture_set: i32,
    /// The metallic-roughness texture.
    pub metallic_roughness_texture_set: i32,
    /// Normal texture
    pub normal_texture_set: i32,
    /// Occlusion texture set
    pub occlusion_texture_set: i32,
    /// Emissive texture set
    pub emissive_texture_set: i32,
    /// The factor for the metalness of the material.
    pub metallic_factor: f32,
    /// The factor for the roughness of the material.
    pub roughness_factor: f32,
    /// Alpha mask - see fragment shader
    pub alpha_mask: f32,
    /// Alpha mask cutoff - see fragment shader
    pub alpha_mask_cutoff: f32,
}

impl Material {
    /// Load a material from a glTF document
    pub fn load(
        mesh_name: &str,
        set_layout: vk::DescriptorSetLayout,
        material: MaterialData,
        vulkan_context: &VulkanContext,
        _buffer: &[u8],
        images: &[gltf::image::Data],
    ) -> Result<(Self, vk::DescriptorSet)> {
        let material_name = format!(
            "Material {} for mesh {}",
            material.name().unwrap_or("<unnamed>"),
            mesh_name
        );

        let empty_texture = Texture::empty(vulkan_context)?;

        let pbr_metallic_roughness = material.pbr_metallic_roughness();
        let pbr_specular_glossiness = material.pbr_specular_glossiness();

        // Base Colour
        let base_color_texture_info = pbr_metallic_roughness.base_color_texture();
        let base_color_texture_set = get_texture_set(base_color_texture_info.as_ref());
        let base_color_texture = base_color_texture_info
            .map(|i| {
                Texture::load(
                    &format!("Base Colour texture for {}", mesh_name),
                    i.texture(),
                    vulkan_context,
                    images,
                )
            })
            .flatten()
            .unwrap_or_else(|| empty_texture.clone());
        let base_colour_factor = Vector4::from(pbr_metallic_roughness.base_color_factor());

        // Metallic Roughness
        let metallic_roughness_texture_info = pbr_metallic_roughness.metallic_roughness_texture();
        let metallic_roughness_texture_set =
            get_texture_set(metallic_roughness_texture_info.as_ref());
        let metallic_roughness_texture = metallic_roughness_texture_info
            .map(|i| {
                Texture::load(
                    &format!("Metallic Roughness texture for {}", mesh_name),
                    i.texture(),
                    vulkan_context,
                    images,
                )
            })
            .flatten()
            .unwrap_or_else(|| empty_texture.clone());

        // Normal map
        let normal_texture_info = material.normal_texture();
        let normal_texture_set = normal_texture_info
            .as_ref()
            .map(|t| t.tex_coord() as i32)
            .unwrap_or(-1);
        let normal_texture = normal_texture_info
            .map(|i| {
                Texture::load(
                    &format!("Normal texture for {}", mesh_name),
                    i.texture(),
                    vulkan_context,
                    images,
                )
            })
            .flatten()
            .unwrap_or_else(|| empty_texture.clone());

        // Occlusion
        let occlusion_texture_info = material.occlusion_texture();
        let occlusion_texture_set = occlusion_texture_info
            .as_ref()
            .map(|t| t.tex_coord() as i32)
            .unwrap_or(-1);
        let occlusion_texture = occlusion_texture_info
            .map(|i| {
                Texture::load(
                    &format!("Occlusion texture for {}", mesh_name),
                    i.texture(),
                    vulkan_context,
                    images,
                )
            })
            .flatten()
            .unwrap_or_else(|| empty_texture.clone());

        // Emission
        let emissive_texture_info = material.emissive_texture();
        let emissive_texture = emissive_texture_info
            .as_ref()
            .map(|i| {
                Texture::load(
                    &format!("Occlusion texture for {}", mesh_name),
                    i.texture(),
                    vulkan_context,
                    images,
                )
            })
            .flatten()
            .unwrap_or_else(|| empty_texture.clone());
        let emissive_texture_set = emissive_texture_info
            .map(|t| t.tex_coord() as i32)
            .unwrap_or(-1);

        // Factors
        let emmissive_factor = vector![0., 0., 0., 0.];
        let diffuse_factor = pbr_specular_glossiness
            .as_ref()
            .map(|p| Vector4::from(p.diffuse_factor()))
            .unwrap_or_else(Vector4::zeros);
        let specular_factor = pbr_specular_glossiness
            .as_ref()
            .map(|p| arr_to_vec4(p.specular_factor()))
            .unwrap_or_else(Vector4::zeros);
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
        let workflow = if pbr_specular_glossiness.is_some() {
            1.
        } else {
            0.
        };

        // Descriptor set
        let descriptor_set = vulkan_context.create_textures_descriptor_sets(
            set_layout,
            &material_name,
            &[
                &base_color_texture,
                &metallic_roughness_texture,
                &normal_texture,
                &occlusion_texture,
                &emissive_texture,
            ],
        )?[0];

        Ok((
            Material {
                base_colour_factor,
                emmissive_factor,
                diffuse_factor,
                specular_factor,
                workflow,
                base_color_texture_set,
                metallic_roughness_texture_set,
                normal_texture_set,
                occlusion_texture_set,
                emissive_texture_set,
                metallic_factor,
                roughness_factor,
                alpha_mask,
                alpha_mask_cutoff,
            },
            descriptor_set,
        ))
    }
}

fn arr_to_vec4(vec3: [f32; 3]) -> Vector4<f32> {
    vector![vec3[0], vec3[1], vec3[2], 0.]
}

fn get_texture_set(info: Option<&Info>) -> i32 {
    info.map(|t| t.tex_coord() as i32).unwrap_or(-1)
}
