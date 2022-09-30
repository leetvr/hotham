use hotham::components::{stage, LocalTransform};
use hotham::glam::Affine3A;
use hotham::rapier3d::prelude::RigidBodyType;
use hotham::systems::pointers::{POSITION_OFFSET, ROTATION_OFFSET};
use hotham::Engine;
use hotham::{
    asset_importer::{add_model_to_world, Models},
    contexts::{InputContext, PhysicsContext},
    hecs::{Entity, With, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
};

use crate::components::{Color, Saber};

const SABER_HEIGHT: f32 = 0.8;
const SABER_HALF_HEIGHT: f32 = SABER_HEIGHT / 2.;
const SABER_WIDTH: f32 = 0.02;
const SABER_HALF_WIDTH: f32 = SABER_WIDTH / 2.;

/// Sync the postion of the player's sabers with the position of their controllers in OpenXR
pub fn sabers_system(engine: &mut Engine) {
    sabers_system_inner(&mut engine.world, &engine.input_context)
}

fn sabers_system_inner(world: &mut World, input_context: &InputContext) {
    // Get the isometry of the stage
    let global_from_stage = stage::get_global_from_stage(world);

    // Create a transform from local space to grip space.
    let grip_from_local = Affine3A::from_rotation_translation(ROTATION_OFFSET, POSITION_OFFSET);

    for (_, (color, local_transform)) in
        world.query_mut::<With<Saber, (&Color, &mut LocalTransform)>>()
    {
        // Get our the space and path of the hand.
        let stage_from_grip = match color {
            Color::Red => input_context.left.stage_from_grip(),
            Color::Blue => input_context.right.stage_from_grip(),
        };

        // Apply transform
        let position = global_from_stage * stage_from_grip * grip_from_local;
        local_transform.update_from_affine(&position);
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
    let components = physics_context.create_rigid_body_and_collider(saber, rigid_body, collider);
    world.insert(saber, components).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sabers() {
        use hotham::components::{GlobalTransform, LocalTransform};

        let mut world = World::new();
        let input_context = InputContext::testing();
        let saber = world.spawn((
            Color::Red,
            Saber {},
            LocalTransform::default(),
            GlobalTransform::default(),
        ));
        sabers_system_inner(&mut world, &input_context);

        let local_transform = world.get::<LocalTransform>(saber).unwrap();
        approx::assert_relative_eq!(
            local_transform.translation,
            [-0.2, 1.3258567, -0.47001815].into()
        );
    }
}
