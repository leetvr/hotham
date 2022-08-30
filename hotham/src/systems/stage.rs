use crate::{
    components::{GlobalTransform, LocalTransform, RigidBody, Stage},
    hecs::{Entity, World},
    rapier3d::prelude::RigidBodyBuilder,
    resources::PhysicsContext,
};

/// Setup Stage entities to track player's frame of reference in global space
pub fn add_stage(world: &mut World, physics_context: &mut PhysicsContext) -> Entity {
    let rigid_body = {
        let rigid_body = RigidBodyBuilder::fixed().build();
        RigidBody {
            handle: physics_context.rigid_bodies.insert(rigid_body),
        }
    };

    world.spawn((
        Stage {},
        GlobalTransform::default(),
        LocalTransform::default(),
        rigid_body,
    ))
}
