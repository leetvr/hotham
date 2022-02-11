use hotham::nalgebra::{Isometry3, Quaternion, Translation3, UnitQuaternion, Vector3};
use hotham::{
    components::RigidBody,
    gltf_loader::{add_model_to_world, Models},
    hecs::{Entity, PreparedQuery, With, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{vulkan_context::VulkanContext, PhysicsContext, RenderContext, XrContext},
    util::{is_space_valid, posef_to_isometry},
};

use crate::components::{Colour, Saber};

const POSITION_OFFSET: [f32; 3] = [0., 0.071173, -0.066082];
const ROTATION_OFFSET: Quaternion<f32> = Quaternion::new(
    -0.5581498959847122,
    0.8274912503663805,
    0.03413791007514528,
    -0.05061153302400824,
);

const SABER_HEIGHT: f32 = 0.8;
const SABER_HALF_HEIGHT: f32 = SABER_HEIGHT / 2.;
const SABER_WIDTH: f32 = 0.02;
const SABER_HALF_WIDTH: f32 = SABER_WIDTH / 2.;

pub fn sabers_system(
    query: &mut PreparedQuery<With<Saber, (&Colour, &RigidBody)>>,
    world: &mut World,
    xr_context: &XrContext,
    physics_context: &mut PhysicsContext,
) {
    for (_, (colour, rigid_body)) in query.query_mut(world) {
        // Get our the space and path of the hand.
        let time = xr_context.frame_state.predicted_display_time;
        let (space, _) = match colour {
            Colour::Red => (
                &xr_context.left_hand_space,
                xr_context.left_hand_subaction_path,
            ),
            Colour::Blue => (
                &xr_context.right_hand_space,
                xr_context.right_hand_subaction_path,
            ),
        };

        // Locate the hand in the space.
        let space = space.locate(&xr_context.reference_space, time).unwrap();
        if !is_space_valid(&space) {
            return;
        }

        let pose = space.pose;

        // apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body.handle)
            .unwrap();

        let mut position = posef_to_isometry(pose);
        apply_grip_offset(&mut position);

        rigid_body.set_next_kinematic_position(position);
    }
}

pub fn add_saber(
    colour: Colour,
    models: &Models,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) -> Entity {
    let model_name = match colour {
        Colour::Blue => "Blue Saber",
        Colour::Red => "Red Saber",
    };
    let saber = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
    add_saber_physics(world, physics_context, saber);
    world.insert(saber, (Saber {}, colour)).unwrap();
    saber
}

fn add_saber_physics(world: &mut World, physics_context: &mut PhysicsContext, saber: Entity) {
    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cylinder(SABER_HALF_HEIGHT, SABER_HALF_WIDTH)
        .translation([0., SABER_HALF_HEIGHT, 0.].into())
        .sensor(true)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();

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
            components::{Transform, TransformMatrix},
            resources::{PhysicsContext, XrContext},
            systems::update_rigid_body_transforms_system,
        };

        let mut world = World::new();
        let path = std::path::Path::new("../../openxr_loader.dll");
        let (xr_context, _) = XrContext::new_from_path(path).unwrap();
        let mut physics_context = PhysicsContext::default();
        let saber = world.spawn((
            Colour::Red,
            Saber {},
            Transform::default(),
            TransformMatrix::default(),
        ));
        add_saber_physics(&mut world, &mut physics_context, saber);

        let mut saber_query = Default::default();
        let mut rigid_body_transforms_query = Default::default();

        sabers_system(
            &mut saber_query,
            &mut world,
            &xr_context,
            &mut physics_context,
        );
        physics_context.update();
        update_rigid_body_transforms_system(
            &mut rigid_body_transforms_query,
            &mut world,
            &physics_context,
        );

        let transform = world.get::<Transform>(saber).unwrap();
        approx::assert_relative_eq!(transform.translation, [-0.2, 1.328827, -0.433918].into());
    }

    #[test]
    fn test_add_offset() {
        let q1 = UnitQuaternion::from_quaternion(Quaternion::new(
            4.329780281177467e-17,
            0.7071067811865476,
            4.329780281177466e-17,
            0.7071067811865475,
        ));
        let t = Translation3::new(0.2, 1.4, 2.);
        let mut position = Isometry3::from_parts(t, q1);
        apply_grip_offset(&mut position);

        let expected_rotation = Quaternion::new(
            -0.5493369162990798,
            -0.4188107240790279,
            0.6209124327141259,
            -0.3705324286596844,
        );
        let expected_translation = Translation3::new(0.2, 1.328827, 2.066082);

        approx::assert_relative_eq!(position.rotation.quaternion(), &expected_rotation);
        approx::assert_relative_eq!(position.translation, &expected_translation);
    }
}
