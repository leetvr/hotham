use rapier3d::prelude::RigidBodyHandle;

/// Component added to an entity to map it to `rapier` `RigidBody`
#[derive(Debug, Clone)]
pub struct RigidBody {
    /// Handle to the `rapier` `RigidBody`
    pub handle: RigidBodyHandle,
}

impl RigidBody {
    pub fn new(handle: RigidBodyHandle) -> Self {
        Self { handle }
    }
}
