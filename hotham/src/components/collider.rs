use hecs::Entity;
use rapier3d::prelude::ColliderHandle;

/// A component that adds a `rapier` collider to an entity.
/// Essentially a wrapper around `ColliderHandle`
#[derive(Debug, Clone)]
pub struct Collider {
    /// A list of entities that may have collided with this one this frame
    pub collisions_this_frame: Vec<Entity>,
    /// Handle to the `rapier` Collider
    pub handle: ColliderHandle,
}

impl Collider {
    /// Create a new collider
    pub fn new(handle: ColliderHandle) -> Collider {
        Collider {
            collisions_this_frame: vec![],
            handle,
        }
    }
}
