use crate::{asset_importer::ImportContext, rendering::primitive::Primitive};
/// Wrapper that encapsulates geometry. Maps closely to the [glTF spec](https://www.khronos.org/registry/glTF/specs/2.0/glTF-2.0.html#meshes)
/// Usually automatically added by `gltf_loader`.
#[derive(Debug, Clone, PartialEq)]
pub struct MeshData {
    /// The primitives in this mesh (eg. actual geometry)
    pub primitives: Vec<Primitive>,
}

impl MeshData {
    /// Create new MeshData from a list of Primitives
    pub fn new(primitives: Vec<Primitive>) -> Self {
        Self { primitives }
    }

    /// Takes mesh data from a glTF file, uploads it to the GPU and inserts it into mesh_map
    pub(crate) fn load(mesh: gltf::Mesh, import_context: &mut ImportContext) -> Self {
        let mesh_name = mesh
            .name()
            .map(|s| s.to_string())
            .unwrap_or(format!("Mesh {}", mesh.index()));

        let primitives = mesh
            .primitives()
            .map(|p| Primitive::load(p, import_context, &mesh_name))
            .collect::<Vec<_>>();

        MeshData { primitives }
    }
}
