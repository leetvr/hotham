/// The Hologram component determines whether a given entity is rendered as a hologram or a regular mesh.
///
///
/// Basic usage:
/// ```ignore
/// use hotham::components::Hologram;
/// world.insert_one(entity, Hologram {});
/// ```

#[derive(Debug, Clone, Copy)]
pub struct Hologram {}
