use crate::rendering::mesh_data::MeshData;
use glam::Mat4;
use id_arena::Id;

/// The Hologram component determines whether a given entity is rendered as a hologram or a regular mesh.
///
///
/// Basic usage:
/// ```ignore
/// use hotham::components::Hologram;
/// world.insert_one(entity, Hologram {});
/// ```

#[derive(Debug, Clone, Copy)]
pub struct Hologram {
    pub mesh_data_handle: Id<MeshData>,
    pub hologram_data: HologramData,
}

#[derive(Debug, Clone, Copy)]
pub struct HologramData {
    pub surface_q_in_local: Mat4,
    pub bounds_q_in_local: Mat4,
    pub uv_from_local: Mat4,
}
