use rapier3d::prelude::RigidBodyType as RapierBodyType;

/// A component used to synchronise this entity's position in the game simulation with the physics simulation.
///
/// You can indicate to [`crate::systems::physics_system`] how you'd like this entity to be treated by changing the `body_type` field
/// . Setting the `body_type` to [`BodyType::Dynamic`] will result in the entity having its [`crate::components::GlobalTransform`]
/// overwritten by its position in the physics simulation - any updates to [`crate::components::LocalTransform`] or [`crate::components::GlobalTransform`] will be overwritten.
///
/// Any other kind of body is treated as *game controlled* - that is, updating the entity's [`crate::components::LocalTransform`] will not be overwritten
/// and the position of the entity in the physics simulation will be updated based on its [`crate::components::GlobalTransform`] (all transforms in the
/// physics simulation are in global space).
///
/// ## Panics
///
/// Trying to create a [`RigidBody`] with a `body_type` of [`BodyType::Dynamic`], or change an existing [`RigidBody`]'s `body_type` to
/// be [`BodyType::Dynamic`] on a [`hecs::Entity`] that has a [`Parent`] component will cause a panic. Don't do it.
#[derive(Debug, Clone)]
pub struct RigidBody {
    pub body_type: BodyType,
    pub linear_velocity: glam::Vec3,
    pub angular_velocity: glam::Vec3,
    pub mass: f32,
    pub lock_rotations: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyType {
    KinematicPositionBased,
    KinematicVelocityBased,
    Dynamic,
    Fixed,
}

impl From<BodyType> for RapierBodyType {
    fn from(r: BodyType) -> Self {
        match r {
            BodyType::KinematicPositionBased => RapierBodyType::KinematicPositionBased,
            BodyType::KinematicVelocityBased => RapierBodyType::KinematicVelocityBased,
            BodyType::Dynamic => RapierBodyType::Dynamic,
            BodyType::Fixed => RapierBodyType::Fixed,
        }
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            body_type: BodyType::Dynamic,
            linear_velocity: Default::default(),
            angular_velocity: Default::default(),
            mass: 0.,
            lock_rotations: false,
        }
    }
}

impl RigidBody {
    pub fn kinematic_position_based() -> Self {
        Self {
            body_type: BodyType::KinematicPositionBased,
            ..Default::default()
        }
    }
}
