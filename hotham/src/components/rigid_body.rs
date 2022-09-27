use rapier3d::prelude::RigidBodyHandle;

/// A component used to synchronise this entity's position in the game simulation with the physics simulation.
///
/// There are two ways to synchronize between the physics simulation and the game:
///
/// 1. **Game controlled** - this entity will have its rigid body position set by the **game** simulation
/// 1. **Physics controlled** - this entity will have its position set by the **physics** simulation
///
/// You can indicate to [`crate::systems::physics_system`] how you'd like this entity to be treated by adding or removing
/// the [`super::PhysicsControlled`] component to an entity. If an entity has the [`super::PhysicsControlled`] component, you
/// are indicating that you want this entity's position in the game simulation to be entirely controlled
/// by the physics simulation.
///
/// There are a couple of situations where this may not make sense (eg. if a rigid body is kinematic position based)
/// so it is up to you to pick the right body type or you'll have a Very Bad Time.
///
/// Physics controlled objects will have their [`crate::components::LocalTransform`] updated directly. What this means
/// is that the entity should *NOT* have a `Parent*, or else its position in the game simulation will not be updated
/// and you will have a Very Bad Time.
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
