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
    /// Takes mesh data from a glTF file, uploads it to the GPU and inserts it into mesh_map
    pub(crate) fn load(mesh: gltf::Mesh, import_context: &mut ImportContext) {
        let mesh_name = mesh
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Mesh {}", mesh.index()));
        let index = mesh.index();

        let primitives = mesh
            .primitives()
            .map(|p| Primitive::load(p, import_context, &mesh_name))
            .collect::<Vec<_>>();

        let mesh_data = MeshData { primitives };
        let handle = import_context
            .render_context
            .resources
            .mesh_data
            .alloc(mesh_data);

        let mesh = Mesh { handle };
        import_context.mesh_map.insert(index, mesh);
    }
}
