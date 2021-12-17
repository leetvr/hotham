use hotham::{
    components::RigidBody,
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{PhysicsContext, XrContext},
    util::posef_to_isometry,
};
use legion::{system, Entity, World};
use nalgebra::{vector, Isometry3, Quaternion, Translation3, UnitQuaternion};

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

#[system(for_each)]
pub fn sabers(
    _: &Saber,
    colour: &Colour,
    rigid_body_component: &RigidBody,
    #[resource] xr_context: &XrContext,
    #[resource] physics_context: &mut PhysicsContext,
) {
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
    let pose = space
        .locate(&xr_context.reference_space, time)
        .unwrap()
        .pose;

    // apply transform
    let rigid_body = physics_context
        .rigid_bodies
        .get_mut(rigid_body_component.handle)
        .unwrap();

    let mut position = posef_to_isometry(pose);
    apply_offset(&mut position);
    rigid_body.set_next_kinematic_position(position);
}

pub fn add_saber_physics(world: &mut World, physics_context: &mut PhysicsContext, saber: Entity) {
    let mut saber_entry = world.entry(saber).unwrap();

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cylinder(SABER_HALF_HEIGHT, SABER_HALF_WIDTH)
        .translation(vector![-SABER_WIDTH, SABER_HALF_HEIGHT, 0.])
        .sensor(true)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
    let (collider, rigid_body) =
        physics_context.add_rigid_body_and_collider(saber, rigid_body, collider);
    saber_entry.add_component(collider);
    saber_entry.add_component(rigid_body);
}

fn apply_offset(position: &mut Isometry3<f32>) {
    let updated_rotation = position.rotation.quaternion() * ROTATION_OFFSET;
    let updated_translation = position.translation.vector
        - vector!(POSITION_OFFSET[0], POSITION_OFFSET[1], POSITION_OFFSET[2]);
    position.rotation = UnitQuaternion::from_quaternion(updated_rotation);
    position.translation = Translation3::from(updated_translation);
}

#[cfg(test)]
mod tests {
    use super::*;
    use hotham::{
        components::{Transform, TransformMatrix},
        resources::{PhysicsContext, XrContext},
        schedule_functions::physics_step,
        systems::update_rigid_body_transforms_system,
    };
    use legion::{IntoQuery, Resources, Schedule, World};
    use nalgebra::{vector, Quaternion, Translation3};

    #[test]
    fn test_sabers() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let path = std::path::Path::new("../../openxr_loader.dll");
        let (xr_context, _) = XrContext::new_from_path(path).unwrap();
        let mut physics_context = PhysicsContext::default();
        let saber = world.push((
            Colour::Red,
            Saber {},
            Transform::default(),
            TransformMatrix::default(),
        ));
        add_saber_physics(&mut world, &mut physics_context, saber);

        resources.insert(xr_context);
        resources.insert(physics_context);

        let mut schedule = Schedule::builder()
            .add_system(sabers_system())
            .add_thread_local_fn(physics_step)
            .add_system(update_rigid_body_transforms_system())
            .build();

        schedule.execute(&mut world, &mut resources);

        let mut query = <(&Transform, &Saber)>::query();
        let (transform, _) = query.get(&world, saber).unwrap();

        approx::assert_relative_eq!(transform.translation, vector![-0.2, 1.328827, -0.433918]);
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
        apply_offset(&mut position);

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
