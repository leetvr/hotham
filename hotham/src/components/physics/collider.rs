use std::fmt::Debug;

use hecs::Entity;
pub use rapier3d::prelude::{ActiveCollisionTypes, SharedShape};

use crate::contexts::physics_context::{DEFAULT_COLLISION_GROUP, HAND_COLLISION_GROUP};

/// A component that enables collision detection - essentially a thin wrapper around [`rapier3d::prelude::Collider`].
#[derive(Clone)]
pub struct Collider {
    /// A list of entities that may have collided with this one this frame
    pub collisions_this_frame: Vec<Entity>,
    /// The shape of this collider
    pub shape: SharedShape,
    /// Is this a sensor collider?
    pub sensor: bool,
    /// What collision groups is this a member of?
    pub collision_groups: u32,
    /// What groups can this collider interact with?
    pub collision_filter: u32,
    /// What kinds of colliders can this collider interact with?
    pub active_collision_types: ActiveCollisionTypes,
    /// Should this collider be offset from its parent (if it has one)?
    pub offset_from_parent: glam::Vec3,
    /// How "bouncy" is this collider?
    pub restitution: f32,
    /// What is the mass of this collider?
    pub mass: f32,
    /// What's the friction co-efficient of this collider?
    pub friction: f32,
}

impl Debug for Collider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Collider")
            .field("collisions_this_frame", &self.collisions_this_frame)
            .field("shape", &self.shape.shape_type())
            .field("sensor", &self.sensor)
            .field("collision_groups", &self.collision_groups)
            .field("collision_filter", &self.collision_filter)
            .field("active_collision_types", &self.active_collision_types)
            .field("offset_from_parent", &self.offset_from_parent)
            .field("restitution", &self.restitution)
            .field("mass", &self.mass)
            .field("friction", &self.friction)
            .finish()
    }
}

impl Collider {
    /// Create a new collider
    pub fn new(shape: SharedShape) -> Collider {
        Collider {
            shape,
            ..Default::default()
        }
    }

    /// Create a new sensor collider
    pub fn sensor(shape: SharedShape) -> Collider {
        Collider {
            shape,
            sensor: true,
            collision_groups: u32::MAX,
            active_collision_types: ActiveCollisionTypes::all(),
            collision_filter: u32::MAX,
            ..Default::default()
        }
    }
}

impl Default for Collider {
    fn default() -> Self {
        Self {
            collisions_this_frame: Default::default(),
            shape: SharedShape::ball(1.0),
            sensor: false,
            collision_groups: DEFAULT_COLLISION_GROUP,
            collision_filter: DEFAULT_COLLISION_GROUP | HAND_COLLISION_GROUP,
            active_collision_types: ActiveCollisionTypes::default(),
            offset_from_parent: Default::default(),
            restitution: 0.5,
            mass: 0.,
            friction: 0.5,
        }
    }
}
