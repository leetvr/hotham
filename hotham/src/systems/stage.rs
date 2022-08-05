use crate::{
    components::{GlobalTransform, LocalTransform, RigidBody, Stage},
    hecs::{Entity, With, World},
    rapier3d::prelude::RigidBodyBuilder,
    resources::PhysicsContext,
    util::{isometry_to_posef, matrix_to_isometry, posef_to_isometry},
    xr,
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

/// Update player's views to take into account the current position of the Stage in global space
///
/// Must happen each tick after parent transforms have been updated.
pub fn update_views_with_stage_transform(world: &mut World, views: &mut [xr::View]) {
    let stage_isometry = world
        .query_mut::<With<Stage, &GlobalTransform>>()
        .into_iter()
        .next()
        .map(|(_, global_transform)| matrix_to_isometry(global_transform.0));

    if let Some(stage_isometry) = stage_isometry {
        for view in views {
            view.pose = isometry_to_posef(stage_isometry * posef_to_isometry(view.pose));
        }
    }
}
