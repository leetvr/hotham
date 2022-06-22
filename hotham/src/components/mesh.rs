use anyhow::Result;
use ash::vk;
use nalgebra::Matrix4;

use crate::{
    asset_importer::ImportContext,
    rendering::{buffer::Buffer, primitive::Primitive},
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

/// Component that encapsulates an Entity's geometry. Maps closely to the [glTF spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#meshes)
/// Usually automatically added by `gltf_loader`.
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// The primitives in this mesh (eg. actual geometry)
    pub primitives: Vec<Primitive>,
}

impl Mesh {
    pub(crate) fn load(mesh_data: gltf::Mesh, import_context: &mut ImportContext) {
        let mesh_name = mesh_data
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Mesh {}", mesh_data.index()));

        let primitives = mesh_data
            .primitives()
            .map(|p| Primitive::load(p, import_context, &mesh_name))
            .collect::<Vec<_>>();

        let mesh = Mesh { primitives };
    }
}
