use hecs::{PreparedQuery, World};
use rapier3d::prelude::RigidBodyType;

use crate::{
    components::{Collider, Hand, RigidBody},
    resources::PhysicsContext,
};

pub fn grabbing_system(
    query: &PreparedQuery<(&mut Hand, &Collider)>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    for (_, (hand, collider)) in query.query_mut(world) {
        // Check to see if we are currently gripping
        if hand.grip_value >= 1.0 {
            // If we already have a grabbed entity, no need to do anything.
            if hand.grabbed_entity.is_some() {
                return;
            };

            // Check to see if we are colliding with an entity
            if let Some(other_entity) = collider.collisions_this_frame.first() {
                let rigid_body_handle = world.get::<&RigidBody>(*other_entity).unwrap().handle;
                let rigid_body = physics_context
                    .rigid_bodies
                    .get_mut(rigid_body_handle)
                    .unwrap();

                // Set its body type to kinematic position based so it can be updated with the hand
                rigid_body.set_body_type(RigidBodyType::KinematicPositionBased);

                // Store a reference to the grabbed entity
                hand.grabbed_entity.replace(*other_entity);
            }
        } else {
            // If we are not gripping, but we have a grabbed entity, release it
            if let Some(grabbed_entity) = hand.grabbed_entity.take() {
                let rigid_body_handle = world.get::<&RigidBody>(grabbed_entity).unwrap().handle;
                let rigid_body = physics_context
                    .rigid_bodies
                    .get_mut(rigid_body_handle)
                    .unwrap();

                // Set its body type back to dynamic
                rigid_body.set_body_type(RigidBodyType::Dynamic);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use crate::{
        components::{hand::Handedness, Info, Transform},
        resources::PhysicsContext,
        systems::update_rigid_body_transforms_system,
    };

    #[test]
    fn test_grabbing_system() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        let grabbed_collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0).build();
        let grabbed_rigid_body = RigidBodyBuilder::new_dynamic().build();
        let grabbed_entity = world.spawn((Info {
            name: "Test entity".to_string(),
            node_id: 0,
        },));
        let components = physics_context.get_rigid_body_and_collider(
            grabbed_entity,
            grabbed_rigid_body,
            grabbed_collider,
        );
        world.insert(grabbed_entity, components);

        // Fully gripped hand
        let hand = Hand {
            handedness: Handedness::Left,
            grip_value: 1.0,
            grabbed_entity: None,
        };

        // Collider
        let hand_collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0).build();
        let handle = physics_context.colliders.insert(hand_collider);
        let collider = Collider {
            handle,
            collisions_this_frame: vec![grabbed_entity],
        };

        let hand_entity = world.spawn((hand, collider));

        let query = PreparedQuery::<(&mut Hand, &Collider)>::default();
        let rigid_body_query = PreparedQuery::<(&RigidBody, &mut Transform)>::default();

        let mut schedule = || {
            grabbing_system(&query, &mut world, &mut physics_context);
            physics_context.update();
            update_rigid_body_transforms_system(
                &mut rigid_body_query,
                &mut world,
                &mut physics_context,
            );
        };

        schedule();

        let hand = world.get_mut::<&Hand>(grabbed_entity).unwrap();
        assert_eq!(hand.grabbed_entity.unwrap(), grabbed_entity);
        hand.grip_value = 0.0;

        schedule();

        let hand = world.get_mut::<&Hand>(grabbed_entity).unwrap();
        assert_eq!(hand.grabbed_entity.unwrap(), grabbed_entity);
        assert!(hand.grabbed_entity.is_none());
    }
}
