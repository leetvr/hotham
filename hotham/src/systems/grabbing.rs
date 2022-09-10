use hecs::{PreparedQuery, World};
use rapier3d::prelude::RigidBodyType;

use crate::{
    components::{Collider, Grabbable, Hand, RigidBody},
    resources::PhysicsContext,
};

/// Grabbing system
/// Used to allow a player to grab objects. Used in conjunction with `hands_system`
pub fn grabbing_system(
    query: &mut PreparedQuery<(&mut Hand, &Collider)>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    puffin::profile_function!();
    for (_, (hand, collider)) in query.query(world).iter() {
        // Check to see if we are currently gripping
        if hand.grip_value >= 1.0 {
            // If we already have a grabbed entity, no need to do anything.
            if hand.grabbed_entity.is_some() {
                return;
            };

            // Check to see if we are colliding with an entity
            for other_entity in collider.collisions_this_frame.iter() {
                if world.get::<Grabbable>(*other_entity).is_ok() {
                    let rigid_body_handle = world.get::<RigidBody>(*other_entity).unwrap().handle;
                    let rigid_body = physics_context
                        .rigid_bodies
                        .get_mut(rigid_body_handle)
                        .unwrap();

                    // Set its body type to kinematic position based so it can be updated with the hand
                    rigid_body.set_body_type(RigidBodyType::KinematicPositionBased);

                    // Store a reference to the grabbed entity
                    hand.grabbed_entity.replace(*other_entity);

                    break;
                }
            }
        } else {
            // If we are not gripping, but we have a grabbed entity, release it
            if let Some(grabbed_entity) = hand.grabbed_entity.take() {
                let rigid_body_handle = world.get::<RigidBody>(grabbed_entity).unwrap().handle;
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
        components::{hand::Handedness, Info, LocalTransform},
        resources::PhysicsContext,
        systems::update_local_transform_with_rigid_body_system,
    };

    #[test]
    fn test_grabbing_system() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        let grabbed_collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0).build();
        let grabbed_rigid_body = RigidBodyBuilder::new(RigidBodyType::Dynamic).build();
        let grabbed_entity = world.spawn((
            Info {
                name: "Test entity".to_string(),
                node_id: 0,
            },
            Grabbable {},
        ));
        let components = physics_context.get_rigid_body_and_collider(
            grabbed_entity,
            grabbed_rigid_body,
            grabbed_collider,
        );
        world.insert(grabbed_entity, components).unwrap();

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

        let mut query = Default::default();
        let mut rigid_body_query = Default::default();

        schedule(
            &mut query,
            &mut world,
            &mut physics_context,
            &mut rigid_body_query,
        );

        let mut hand = world.get_mut::<Hand>(hand_entity).unwrap();
        assert_eq!(hand.grabbed_entity.unwrap(), grabbed_entity);
        hand.grip_value = 0.0;
        drop(hand);

        schedule(
            &mut query,
            &mut world,
            &mut physics_context,
            &mut rigid_body_query,
        );

        let mut hand = world.get_mut::<Hand>(hand_entity).unwrap();
        assert!(hand.grabbed_entity.is_none());

        // Make sure hand can't grip colliders *without* a Grabbable component
        hand.grip_value = 1.0;
        drop(hand);
        world.remove::<(Grabbable,)>(grabbed_entity).unwrap();

        schedule(
            &mut query,
            &mut world,
            &mut physics_context,
            &mut rigid_body_query,
        );

        let hand = world.get::<Hand>(hand_entity).unwrap();
        assert!(hand.grabbed_entity.is_none());
    }

    fn schedule(
        query: &mut PreparedQuery<(&mut Hand, &Collider)>,
        world: &mut World,
        physics_context: &mut PhysicsContext,
        rigid_body_query: &mut PreparedQuery<(&RigidBody, &mut LocalTransform)>,
    ) {
        grabbing_system(query, world, physics_context);
        physics_context.update();
        update_local_transform_with_rigid_body_system(rigid_body_query, world, physics_context);
    }
}
