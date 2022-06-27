use crate::{asset_importer::ImportContext, rendering::mesh_data::MeshData};
use id_arena::Id;

#[derive(Debug, Clone)]
pub struct Mesh {
    pub handle: Id<MeshData>,
}

impl Mesh {
    /// Takes mesh data from a glTF file, uploads it to the GPU and inserts it into mesh_map
    pub(crate) fn load(mesh_data: gltf::Mesh, import_context: &mut ImportContext) {
        MeshData::load(mesh_data, import_context)
    }
}
