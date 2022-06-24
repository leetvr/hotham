use anyhow::Result;
use ash::vk;
use nalgebra::Matrix4;

use crate::{
    asset_importer::ImportContext,
    components::Mesh,
    rendering::{buffer::Buffer, primitive::Primitive},
    resources::{render_context::DescriptorSetLayouts, VulkanContext},
};
use std::mem::{transmute, MaybeUninit};
/// Wrapper that encapsulates geometry. Maps closely to the [glTF spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#meshes)
/// Usually automatically added by `gltf_loader`.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    /// The primitives in this mesh (eg. actual geometry)
    pub primitives: Vec<Primitive>,
}

impl MeshData {
    /// Takes mesh data from a glTF file, uploads it to the GPU and hands back a handle
    pub(crate) fn load(mesh_data: gltf::Mesh, import_context: &mut ImportContext) -> Mesh {
        let mesh_name = mesh_data
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Mesh {}", mesh_data.index()));

        let primitives = mesh_data
            .primitives()
            .map(|p| Primitive::load(p, import_context, &mesh_name))
            .collect::<Vec<_>>();

        let mesh_data = MeshData { primitives };

        todo!()
    }
}
