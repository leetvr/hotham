use crate::{
    asset_importer::ImportContext, rendering::mesh_data::MeshData, resources::RenderContext,
};
use id_arena::Id;

/// A handle to MeshData stored on the GPU.
#[derive(Debug, Clone)]
pub struct Mesh {
    pub handle: Id<MeshData>,
}

impl Mesh {
    /// Create a new mesh handle from mesh data.
    pub fn new(mesh_data: MeshData, render_context: &mut RenderContext) -> Self {
        let handle = render_context.resources.mesh_data.alloc(mesh_data);
        Mesh { handle }
    }

    /// Takes mesh data from a glTF file, uploads it to the GPU and inserts it into mesh_map
    pub(crate) fn load(gltf_mesh_data: gltf::Mesh, import_context: &mut ImportContext) {
        let index = gltf_mesh_data.index();
        let mesh_data = MeshData::load(gltf_mesh_data, import_context);

        import_context
            .mesh_map
            .insert(index, Mesh::new(mesh_data, import_context.render_context));
    }
}
