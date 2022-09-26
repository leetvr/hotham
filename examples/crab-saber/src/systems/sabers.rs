use hotham::components::Stage;
use hotham::nalgebra::{Quaternion, Translation3, UnitQuaternion};
use hotham::rapier3d::prelude::RigidBodyType;
use hotham::Engine;
use hotham::{
    asset_importer::{add_model_to_world, Models},
    components::RigidBody,
    contexts::{InputContext, PhysicsContext},
    hecs::{Entity, With, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
};

use crate::components::{Color, Saber};

const POSITION_OFFSET: Translation3<f32> = Translation3::new(0., -0.08, 0.);
const ROTATION_OFFSET: UnitQuaternion<f32> = UnitQuaternion::new_unchecked(Quaternion::<f32>::new(
    -0.558_149_9,
    0.827_491_2,
    0.034_137_91,
    -0.050_611_533,
));

const SABER_HEIGHT: f32 = 0.8;
const SABER_HALF_HEIGHT: f32 = SABER_HEIGHT / 2.;
const SABER_WIDTH: f32 = 0.02;
const SABER_HALF_WIDTH: f32 = SABER_WIDTH / 2.;

/// Sync the postion of the player's sabers with the position of their controllers in OpenXR
pub fn sabers_system(engine: &mut Engine) {
    sabers_system_inner(
        &mut engine.world,
        &engine.input_context,
        &mut engine.physics_context,
    )
}

fn sabers_system_inner(
    world: &mut World,
    input_context: &InputContext,
    physics_context: &mut PhysicsContext,
) {
    // Get the isometry of the stage
    let global_from_stage = world
        .query_mut::<With<Stage, &RigidBody>>()
        .into_iter()
        .next()
        .map_or(Default::default(), |(_, rigid_body)| {
            *physics_context.rigid_bodies[rigid_body.handle].position()
        });

    let grip_from_saber = ROTATION_OFFSET * POSITION_OFFSET;

    for (_, (color, rigid_body)) in world.query_mut::<With<Saber, (&Color, &RigidBody)>>() {
        // Get our the space and path of the hand.
        let stage_from_grip = match color {
            Color::Red => input_context.left.stage_from_grip(),
            Color::Blue => input_context.right.stage_from_grip(),
        };

        // Apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body.handle)
            .unwrap();

        let position = global_from_stage * stage_from_grip * grip_from_saber;

        rigid_body.set_next_kinematic_position(position);
    }
}

pub fn add_saber(
    color: Color,
    models: &Models,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) -> Entity {
    let model_name = match color {
        Color::Blue => "Blue Saber",
        Color::Red => "Red Saber",
    };
    let saber = add_model_to_world(model_name, models, world, physics_context, None).unwrap();
    add_saber_physics(world, physics_context, saber);
    world.insert(saber, (Saber {}, color)).unwrap();
    saber
}

fn add_saber_physics(world: &mut World, physics_context: &mut PhysicsContext, saber: Entity) {
    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cylinder(SABER_HALF_HEIGHT, SABER_HALF_WIDTH)
        .translation([0., SABER_HALF_HEIGHT, 0.].into())
        .sensor(true)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build();

    // Add the components to the entity.
    let components = physics_context.get_rigid_body_and_collider(saber, rigid_body, collider);
    world.insert(saber, components).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn test_sabers() {
        use hotham::{
            components::{GlobalTransform, LocalTransform},
            contexts::{PhysicsContext, XrContext},
        };

        let mut world = World::new();
        let path = std::path::Path::new("../../openxr_loader.dll");
        let (xr_context, _) = XrContext::new_from_path(path).unwrap();
        let mut input_context = InputContext::default();
        let mut physics_context = PhysicsContext::default();
        let saber = world.spawn((
            Color::Red,
            Saber {},
            LocalTransform::default(),
            GlobalTransform::default(),
        ));
        add_saber_physics(&mut world, &mut physics_context, saber);

        input_context.update(&xr_context);
        sabers_system_inner(&mut world, &input_context, &mut physics_context);
        physics_context.update();

        let rigid_body_handle = world.get::<RigidBody>(saber).unwrap().handle;
        let rigid_body = physics_context.rigid_bodies.get(rigid_body_handle).unwrap();
        approx::assert_relative_eq!(
            rigid_body.position().translation,
            [-0.2, 1.3258252, -0.4700315].into()
        );
    }
}
