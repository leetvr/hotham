use hecs::With;
use nalgebra::Isometry3;

use crate::{
    components::{GlobalTransform, LocalTransform, RigidBody, Stage},
    contexts::PhysicsContext,
    hecs::{Entity, World},
    rapier3d::prelude::RigidBodyBuilder,
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

/// Get the transform of the stage in global space.
pub fn get_global_from_stage(world: &mut World) -> Isometry3<f32> {
    // Get the stage transform
    world
        .query_mut::<With<Stage, &GlobalTransform>>()
        .into_iter()
        .next()
        .map(|(_, global_transform)| global_transform.to_isometry())
        .unwrap_or_else(Isometry3::<_>::identity)
}
