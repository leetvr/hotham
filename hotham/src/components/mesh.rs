use anyhow::Result;
use ash::vk;
use nalgebra::Matrix4;

use super::primitive::Primitive;
use crate::{
    buffer::Buffer,
    resources::{render_context::DescriptorSetLayouts, VulkanContext},
};
use std::mem::{transmute, MaybeUninit};

/// Uniform buffer used by the vertex shader for each entity
#[repr(C)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct MeshUBO {
    /// The transform of the entity, in world space
    pub transform: Matrix4<f32>,
    /// Joint matrices for skinning
    pub joint_matrices: [Matrix4<f32>; 128],
    /// The number of joints
    pub joint_count: f32,
}

impl Default for MeshUBO {
    fn default() -> Self {
        let joint_matrices = {
            // Most meshes don't have joints, so no point allocating.
            let mut joint_matrices: [MaybeUninit<Matrix4<f32>>; 128] =
                unsafe { MaybeUninit::uninit().assume_init() };
            for elem in &mut joint_matrices[0..0] {
                elem.write(Matrix4::identity());
            }
            unsafe { transmute::<_, [Matrix4<f32>; 128]>(joint_matrices) }
            //joint_matrices[0] = Matrix4::identity();
        };

        Self {
            transform: Default::default(),
            joint_count: Default::default(),
            joint_matrices,
        }
    }
}

/// Component that encapsulates an Entity's geometry. Maps closesly to the [glTF spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#meshes)
/// Usually automatically added by `gltf_loader`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The descriptor sets for the UBO
    pub descriptor_sets: [vk::DescriptorSet; 1],
    /// UBO sent to the shader
    pub ubo_buffer: Buffer<MeshUBO>,
    /// The actual contents of the UBO
    pub ubo_data: MeshUBO,
    /// The primitives in this mesh (eg. actual geometry)
    pub primitives: Vec<Primitive>,
}

impl Mesh {
    pub(crate) fn load(
        mesh_data: &gltf::Mesh,
        buffer: &[u8],
        vulkan_context: &VulkanContext,
        descriptor_set_layouts: &DescriptorSetLayouts,
        images: &[gltf::image::Data],
    ) -> Result<Mesh> {
        let name = mesh_data.name().unwrap_or("");
        let primitives = mesh_data
            .primitives()
            .map(|p| {
                Primitive::load(
                    descriptor_set_layouts.textures_layout,
                    name,
                    p,
                    buffer,
                    vulkan_context,
                    images,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        // Create descriptor sets
        println!("[HOTHAM_MODEL] Creating descriptor sets for {}", name);
        let descriptor_sets = vulkan_context.create_mesh_descriptor_sets(
            descriptor_set_layouts.mesh_layout,
            mesh_data
                .name()
                .unwrap_or(&format!("Mesh {}", mesh_data.index())),
        )?;
        let descriptor_sets = [descriptor_sets[0]];
        println!("[HOTHAM_MODEL] ..done!");

        let mesh_ubo = MeshUBO::default();
        let ubo_buffer = Buffer::new(
            vulkan_context,
            &[mesh_ubo],
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        vulkan_context.update_buffer_descriptor_set(
            &ubo_buffer,
            descriptor_sets[0],
            0,
            vk::DescriptorType::UNIFORM_BUFFER,
        );

        Ok(Mesh {
            ubo_buffer,
            ubo_data: mesh_ubo,
            descriptor_sets,
            primitives,
        })
    }
}
