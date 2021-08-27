use legion::{system, world::SubWorld, IntoQuery};
use nalgebra::{Quaternion, Translation3, Unit, UnitQuaternion, Vector3};
use rapier3d::math::Isometry;

use crate::{
    components::{hand::Handedness, AnimationController, Hand, RigidBody},
    resources::{PhysicsContext, XrContext},
};

#[system(for_each)]
#[read_component(RigidBody)]
pub fn hands(
    hand: &mut Hand,
    animation_controller: &mut AnimationController,
    rigid_body_component: &RigidBody,
    #[resource] xr_context: &XrContext,
    #[resource] physics_context: &mut PhysicsContext,
    world: &mut SubWorld,
) {
    // Get our the space and path of the hand.
    let time = xr_context.frame_state.predicted_display_time;
    let (space, path) = match hand.handedness {
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

    // TODO: EW. No. No. No.
    let translation: Vector3<f32> = mint::Vector3::from(pose.position).into();
    let translation: Translation3<f32> = Translation3::from(translation);
    let rotation: Quaternion<f32> = mint::Quaternion::from(pose.orientation).into();
    let rotation: UnitQuaternion<f32> = Unit::new_normalize(rotation);
    let position = Isometry {
        rotation,
        translation,
    };
    rigid_body.set_next_kinematic_position(position);

    if let Some(grabbed_entity) = hand.grabbed_entity {
        let mut query = <&RigidBody>::query();
        let handle = query.get(world, grabbed_entity).unwrap().handle;
        let rigid_body = physics_context.rigid_bodies.get_mut(handle).unwrap();
        rigid_body.set_next_kinematic_position(position);
    }

    // get grip value
    let grip_value = openxr::ActionInput::get(&xr_context.grab_action, &xr_context.session, path)
        .unwrap()
        .current_state;

    // Apply to Hand
    hand.grip_value = grip_value;

    // Apply to AnimationController
    animation_controller.blend_amount = grip_value;
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, vec3};
    use legion::{Entity, IntoQuery, Resources, Schedule, World};
    use nalgebra::vector;
    use rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder,
    };

    use super::*;
    use crate::{
        components::Transform, resources::XrContext, schedule_functions::physics_step,
        systems::update_rigid_body_transforms_system,
    };

    #[test]
    pub fn test_hands_system() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let (xr_context, _) = XrContext::new().unwrap();
        let mut physics_context = PhysicsContext::default();

        let hand = add_hand_to_world(&mut physics_context, &mut world, None);

        resources.insert(xr_context);
        resources.insert(physics_context);

        let mut schedule = Schedule::builder()
            .add_system(hands_system())
            .add_thread_local_fn(physics_step)
            .add_system(update_rigid_body_transforms_system())
            .build();

        schedule.execute(&mut world, &mut resources);

        let mut query = <(&Transform, &Hand, &AnimationController)>::query();
        let (transform, hand, animation_controller) = query.get(&world, hand).unwrap();

        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(transform.translation, vec3(-0.2, 1.4, -0.5));
        assert_relative_eq!(animation_controller.blend_amount, 0.0);
    }

    #[test]
    pub fn test_move_grabbed_objects() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let (xr_context, _) = XrContext::new().unwrap();
        let mut physics_context = PhysicsContext::default();

        let grabbed_object_rigid_body = RigidBodyBuilder::new_kinematic_position_based().build(); // grabber sets the rigidbody as kinematic
        let handle = physics_context
            .rigid_bodies
            .insert(grabbed_object_rigid_body);
        let grabbed_entity = world.push((RigidBody { handle }, Transform::default()));

        let _ = add_hand_to_world(&mut physics_context, &mut world, Some(grabbed_entity));

        resources.insert(xr_context);
        resources.insert(physics_context);

        let mut schedule = Schedule::builder()
            .add_system(hands_system())
            .add_thread_local_fn(physics_step)
            .add_system(update_rigid_body_transforms_system())
            .build();

        schedule.execute(&mut world, &mut resources);

        let mut query = <&Transform>::query();
        let transform = query.get(&world, grabbed_entity).unwrap();

        assert_relative_eq!(transform.translation, vec3(-0.2, 1.4, -0.5));
    }

    fn add_hand_to_world(
        physics_context: &mut PhysicsContext,
        world: &mut World,
        grabbed_entity: Option<Entity>,
    ) -> Entity {
        let mut animation_controller = AnimationController::default();
        animation_controller.blend_amount = 100.0; // bogus value

        let mut hand = Hand::left();
        hand.grabbed_entity = grabbed_entity;
        let hand = world.push((animation_controller, hand, Transform::default()));
        {
            let mut hand_entry = world.entry(hand).unwrap();

            // Give it a collider and rigid-body
            let collider = ColliderBuilder::capsule_y(0.05, 0.02)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::INTERSECTION_EVENTS)
                .build();
            let mut rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
            rigid_body.set_next_kinematic_translation(vector![0.0, 1.4, 0.0]);
            let (collider, rigid_body) =
                physics_context.add_rigid_body_and_collider(hand, rigid_body, collider);
            hand_entry.add_component(collider);
            hand_entry.add_component(rigid_body);

            let mut hand = hand_entry.get_component_mut::<Hand>().unwrap();
            hand.grip_value = 100.0; // bogus value
        }

        hand
    }
}
