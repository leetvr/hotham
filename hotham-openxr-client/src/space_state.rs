use core::fmt::Debug;
use openxr_sys::{Quaternionf, Vector3f};

#[derive(Clone)]
pub struct SpaceState {
    pub name: String,
    pub position: Vector3f,
    pub orientation: Quaternionf,
}

impl SpaceState {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            position: Default::default(),
            orientation: Quaternionf::IDENTITY,
        }
    }
}

impl Debug for SpaceState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpaceState")
            .field("name", &self.name)
            .field(
                "position",
                &format!(
                    "x: {}, y: {}, z: {}",
                    self.position.x, self.position.y, self.position.z
                ),
            )
            .field(
                "orientation",
                &format!(
                    "x: {}, y: {}, z: {}, w: {}",
                    self.orientation.x, self.orientation.y, self.orientation.z, self.orientation.w
                ),
            )
            .finish()
    }
}
