use anyhow::Result;
use ash::vk;
use nalgebra::Matrix4;

use super::primitive::Primitive;
use crate::{
    buffer::Buffer,
    resources::{render_context::DescriptorSetLayouts, VulkanContext},
};
use std::mem::MaybeUninit;

#[repr(C)]
#[derive(Debug, Clone, PartialEq, Copy)]
pub struct MeshUBO {
    pub transform: Matrix4<f32>,
    pub joint_matrices: [Matrix4<f32>; 128],
    pub joint_count: f32,
}

impl Default for MeshUBO {
    fn default() -> Self {
        // Most meshes don't have joints, so no point allocating.
        let mut joint_matrices: [Matrix4<f32>; 128] =
            unsafe { MaybeUninit::uninit().assume_init() };
        joint_matrices[0] = Matrix4::identity();
        Self {
            transform: Default::default(),
            joint_count: Default::default(),
            joint_matrices,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub descriptor_sets: [vk::DescriptorSet; 1],
    pub ubo_buffer: Buffer<MeshUBO>,
    pub ubo_data: MeshUBO,
    pub primitives: Vec<Primitive>,
}

impl Mesh {
    pub(crate) fn load(
        mesh_data: &gltf::Mesh,
        buffer: &[u8],
        vulkan_context: &VulkanContext,
        descriptor_set_layouts: &DescriptorSetLayouts,
        _empty_storage_buffer: &Buffer<Matrix4<f32>>,
        images: &Vec<gltf::image::Data>,
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
