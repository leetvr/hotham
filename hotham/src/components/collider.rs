use hecs::Entity;
use rapier3d::prelude::ColliderHandle;

#[derive(Debug, Clone)]
pub struct Collider {
    pub collisions_this_frame: Vec<Entity>,
    pub handle: ColliderHandle,
}

impl Collider {
    pub fn new(handle: ColliderHandle) -> Collider {
        Collider {
            collisions_this_frame: vec![],
            handle,
        }
    }
}
