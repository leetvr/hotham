use hotham::nalgebra::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};
use hotham::rapier3d::prelude::RigidBodyType;
use hotham::{
    asset_importer::{add_model_to_world, Models},
    components::RigidBody,
    hecs::{Entity, PreparedQuery, With, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{InputContext, PhysicsContext},
};

use crate::components::{Color, Saber};

const POSITION_OFFSET: [f32; 3] = [0., 0.071173, -0.066082];
const ROTATION_OFFSET: Quaternion<f32> =
    Quaternion::new(-0.558_149_8, 0.827_491_2, 0.034_137_9, -0.050_611_5);

const SABER_HEIGHT: f32 = 0.8;
const SABER_HALF_HEIGHT: f32 = SABER_HEIGHT / 2.;
const SABER_WIDTH: f32 = 0.02;
const SABER_HALF_WIDTH: f32 = SABER_WIDTH / 2.;

pub fn sabers_system(
    query: &mut PreparedQuery<With<Saber, (&Color, &RigidBody)>>,
    world: &mut World,
    input_context: &InputContext,
    physics_context: &mut PhysicsContext,
) {
    for (_, (color, rigid_body)) in query.query_mut(world) {
        // Get our the space and path of the hand.
        let mut pose = match color {
            Color::Red => input_context.left.grip_pose_local(),
            Color::Blue => input_context.right.grip_pose_local(),
        };

        // apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body.handle)
            .unwrap();

        apply_grip_offset(&mut pose);

        rigid_body.set_next_kinematic_position(pose);
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
    let saber = add_model_to_world(model_name, models, world, None).unwrap();
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

pub fn apply_grip_offset(position: &mut Isometry3<f32>) {
    let updated_rotation = position.rotation.quaternion() * ROTATION_OFFSET;
    let updated_translation = position.translation.vector
        - Vector3::new(POSITION_OFFSET[0], POSITION_OFFSET[1], POSITION_OFFSET[2]);
    position.rotation = UnitQuaternion::from_quaternion(updated_rotation);
    position.translation = Translation3::from(updated_translation);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "windows")]
    #[test]
    fn test_sabers() {
        use hotham::{
            components::{GlobalTransform, LocalTransform},
            resources::{PhysicsContext, XrContext},
            systems::update_local_transform_with_rigid_body_system,
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

        let mut saber_query = Default::default();
        let mut rigid_body_transforms_query = Default::default();

        input_context.update(&xr_context);
        sabers_system(
            &mut saber_query,
            &mut world,
            &input_context,
            &mut physics_context,
        );
        physics_context.update();
        update_local_transform_with_rigid_body_system(
            &mut rigid_body_transforms_query,
            &mut world,
            &physics_context,
        );

        let local_transform = world.get::<LocalTransform>(saber).unwrap();
        approx::assert_relative_eq!(
            local_transform.translation,
            [-0.2, 1.328827, -0.433918].into()
        );
    }

    #[test]
    fn test_add_offset() {
        #[allow(clippy::approx_constant)]
        let q1 = UnitQuaternion::from_quaternion(Quaternion::new(
            4.329_780_3e-17,
            0.707_106_77,
            4.329_780_3e-17,
            0.707_106_77,
        ));
        let t = Translation3::new(0.2, 1.4, 2.);
        let mut position = Isometry3::from_parts(t, q1);
        apply_grip_offset(&mut position);

        let expected_rotation =
            Quaternion::new(-0.549_336_9, -0.418_810_73, 0.620_912_43, -0.370_532_42);
        let expected_translation = Translation3::new(0.2, 1.328827, 2.066082);

        approx::assert_relative_eq!(position.rotation.quaternion(), &expected_rotation);
        approx::assert_relative_eq!(position.translation, &expected_translation);
    }
}
