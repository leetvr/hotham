use hecs::With;
use nalgebra::Matrix4;
use openxr as xr;

use crate::{
    components::{GlobalTransform, LocalTransform, RigidBody, Stage},
    hecs::{Entity, World},
    rapier3d::prelude::RigidBodyBuilder,
    resources::PhysicsContext,
    util::{isometry_to_posef, matrix_to_isometry, posef_to_isometry},
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
pub fn get_global_from_stage(world: &mut World) -> Matrix4<f32> {
    // Get the stage transform
    world
        .query_mut::<With<Stage, &GlobalTransform>>()
        .into_iter()
        .next()
        .map(|(_, global_transform)| global_transform.0)
        .unwrap_or_else(Matrix4::<_>::identity)
}

/// Returns a clone of the views with their poses in global space rather than stage space.
pub fn views_in_global_space(world: &mut World, views: &[xr::View]) -> [xr::View; 2] {
    let stage_isometry = matrix_to_isometry(get_global_from_stage(world));

    let mut views = views.to_owned();
    for view in views.iter_mut() {
        view.pose = isometry_to_posef(stage_isometry * posef_to_isometry(view.pose));
    }
    [views[0], views[1]]
}
