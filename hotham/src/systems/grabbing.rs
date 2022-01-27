use legion::{system, world::SubWorld, EntityStore};
use rapier3d::prelude::RigidBodyType;

use crate::{
    components::{Collider, Hand, RigidBody},
    resources::PhysicsContext,
};

#[system(for_each)]
#[read_component(RigidBody)]
pub fn grabbing(
    hand: &mut Hand,
    collider: &Collider,
    #[resource] physics_context: &mut PhysicsContext,
    world: &SubWorld,
) {
    // Check to see if we are currently gripping
    if hand.grip_value >= 1.0 {
        // If we already have a grabbed entity, no need to do anything.
        if hand.grabbed_entity.is_some() {
            return;
        };

        // Check to see if we are colliding with an entity
        if let Some(grabbed_entity) = collider.collisions_this_frame.first() {
            let entry = world.entry_ref(*grabbed_entity).unwrap();
            let rigid_body_handle = entry.get_component::<RigidBody>().unwrap().handle;
            let rigid_body = physics_context
                .rigid_bodies
                .get_mut(rigid_body_handle)
                .unwrap();

            // Set its body type to kinematic position based so it can be updated with the hand
            rigid_body.set_body_type(RigidBodyType::KinematicPositionBased);

            // Store a reference to the grabbed entity
            hand.grabbed_entity.replace(*grabbed_entity);
        }
    } else {
        // If we are not gripping, but we have a grabbed entity, release it
        if let Some(grabbed_entity) = hand.grabbed_entity.take() {
            let entry = world.entry_ref(grabbed_entity).unwrap();
            let rigid_body_handle = entry.get_component::<RigidBody>().unwrap().handle;
            let rigid_body = physics_context
                .rigid_bodies
                .get_mut(rigid_body_handle)
                .unwrap();

            // Set its body type back to dynamic
            rigid_body.set_body_type(RigidBodyType::Dynamic);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use legion::{Resources, Schedule, World};
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use crate::{
        components::{hand::Handedness, Info},
        resources::PhysicsContext,
        schedule_functions::physics_step,
        systems::update_rigid_body_transforms_system,
    };

    #[test]
    fn test_grabbing_system() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut physics_context = PhysicsContext::default();

        let grabbed_collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0).build();
        let grabbed_rigid_body = RigidBodyBuilder::new_dynamic().build();
        let grabbed_entity = world.push((Info {
            name: "Test entity".to_string(),
            node_id: 0,
        },));
        let (collider, rigid_body) = physics_context.get_rigid_body_and_collider(
            grabbed_entity,
            grabbed_rigid_body,
            grabbed_collider,
        );
        let mut entry = world.entry(grabbed_entity).unwrap();
        entry.add_component(collider);
        entry.add_component(rigid_body);

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

        let hand_entity = world.push((hand, collider));

        resources.insert(physics_context);
        let mut schedule = Schedule::builder()
            .add_system(grabbing_system())
            .add_thread_local_fn(physics_step)
            .add_system(update_rigid_body_transforms_system())
            .build();

        schedule.execute(&mut world, &mut resources);

        let mut entry = world.entry(hand_entity).unwrap();
        let hand = entry.get_component_mut::<Hand>().unwrap();
        assert_eq!(hand.grabbed_entity.unwrap(), grabbed_entity);
        hand.grip_value = 0.0;

        schedule.execute(&mut world, &mut resources);

        let mut entry = world.entry(hand_entity).unwrap();
        let hand = entry.get_component_mut::<Hand>().unwrap();
        assert!(hand.grabbed_entity.is_none());
    }
}
