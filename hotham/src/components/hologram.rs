use nalgebra::Matrix4;

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
    pub surface_q_in_local: Matrix4<f32>,
    pub bounds_q_in_local: Matrix4<f32>,
    pub uv_from_local: Matrix4<f32>,
}