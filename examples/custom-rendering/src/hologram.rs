use hotham::{glam::Mat4, id_arena::Id, rendering::mesh_data::MeshData};

/// The Hologram component is used together with custom rendering to render quadric surfaces.
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
