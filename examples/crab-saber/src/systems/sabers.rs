use hotham::{
    asset_importer::{add_model_to_world, Models},
    components::{
        physics::{BodyType, SharedShape},
        stage, Collider, LocalTransform, RigidBody,
    },
    contexts::InputContext,
    glam::Affine3A,
    hecs::{Entity, With, World},
    systems::pointers::{POSITION_OFFSET, ROTATION_OFFSET},
    Engine,
};

use crate::components::{Color, Saber};

const SABER_HEIGHT: f32 = 0.8;
const SABER_HALF_HEIGHT: f32 = SABER_HEIGHT / 2.;
const SABER_WIDTH: f32 = 0.02;
const SABER_HALF_WIDTH: f32 = SABER_WIDTH / 2.;

/// Sync the transform of the player's sabers with the pose of their controllers in OpenXR
pub fn sabers_system(engine: &mut Engine) {
    sabers_system_inner(&mut engine.world, &engine.input_context)
}

fn sabers_system_inner(world: &mut World, input_context: &InputContext) {
    // Get the isometry of the stage
    let global_from_stage = stage::get_global_from_stage(world);

    // Create a transform from local space to grip space.
    let grip_from_local = Affine3A::from_rotation_translation(ROTATION_OFFSET, POSITION_OFFSET);

    for (_, (color, local_transform)) in
        world.query_mut::<With<(&Color, &mut LocalTransform), &Saber>>()
    {
        // Get our the space and path of the hand.
        let stage_from_grip = match color {
            Color::Red => input_context.left.stage_from_grip(),
            Color::Blue => input_context.right.stage_from_grip(),
        };

        // Apply transform
        let global_from_local = global_from_stage * stage_from_grip * grip_from_local;
        local_transform.update_from_affine(&global_from_local);
    }
}

pub fn add_saber(color: Color, models: &Models, world: &mut World) -> Entity {
    let model_name = match color {
        Color::Blue => "Blue Saber",
        Color::Red => "Red Saber",
    };
    let saber = add_model_to_world(model_name, models, world, None).unwrap();
    add_saber_physics(world, saber);
    world.insert(saber, (Saber {}, color)).unwrap();
    saber
}

fn add_saber_physics(world: &mut World, saber: Entity) {
    // Give it a collider and rigid-body
    let collider = Collider {
        shape: SharedShape::cylinder(SABER_HALF_HEIGHT, SABER_HALF_WIDTH),
        sensor: true,
        ..Default::default()
    };
    let rigid_body = RigidBody {
        body_type: BodyType::KinematicPositionBased,
        ..Default::default()
    };

    // Add the components to the entity.
    world.insert(saber, (collider, rigid_body)).unwrap();
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

        let local_transform = world.get::<&LocalTransform>(saber).unwrap();
        approx::assert_relative_eq!(
            local_transform.translation,
            [-0.2, 1.3258567, -0.47001815].into()
        );
    }
}
