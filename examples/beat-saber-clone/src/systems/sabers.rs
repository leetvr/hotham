use hotham::{
    components::{hand::Handedness, RigidBody},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::{PhysicsContext, XrContext},
    util::posef_to_isometry,
};
use legion::{system, Entity, World};
use nalgebra::{UnitQuaternion, Vector3};

use crate::components::Saber;

#[system(for_each)]
pub fn sabers(
    saber: &mut Saber,
    rigid_body_component: &RigidBody,
    #[resource] xr_context: &XrContext,
    #[resource] physics_context: &mut PhysicsContext,
) {
    // Get our the space and path of the hand.
    let time = xr_context.frame_state.predicted_display_time;
    let (space, _) = match saber.handedness {
        Handedness::Left => (
            &xr_context.left_hand_space,
            xr_context.left_hand_subaction_path,
        ),
        Handedness::Right => (
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
    // position.rotation = position.rotation
    //     * UnitQuaternion::from_axis_angle(&Vector3::y_axis(), std::f32::consts::PI);
    rigid_body.set_next_kinematic_position(position);
}

pub fn add_saber_physics(world: &mut World, physics_context: &mut PhysicsContext, saber: Entity) {
    let mut saber_entry = world.entry(saber).unwrap();

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::capsule_y(0.10, 0.01)
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
    use nalgebra::vector;

    #[test]
    fn test_sabers() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let path = std::path::Path::new("../../openxr_loader.dll");
        let (xr_context, _) = XrContext::new_from_path(path).unwrap();
        let mut physics_context = PhysicsContext::default();
        let saber = world.push((
            Saber {
                handedness: Handedness::Left,
            },
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

        approx::assert_relative_eq!(transform.translation, vector![-0.2, 1.4, -0.5]);
    }
}
