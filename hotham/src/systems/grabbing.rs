use hecs::World;

use crate::{
    components::{
        physics::BodyType, Collider, Grabbable, Grabbed, Hand, Parent, Released, RigidBody,
    },
    Engine,
};

/// Grabbing system
/// Used to allow a player to grab objects. Used in conjunction with `hands_system`
pub fn grabbing_system(engine: &mut Engine) {
    let world = &mut engine.world;
    grabbing_system_inner(world);
}

fn grabbing_system_inner(world: &mut World) {
    // First, clean up any `Released` marker traits from the previous frame. This is important as otherwise
    // any entity that was ever grabbed will still have a `Released` component
    {
        let entities_with_released = world
            .query::<()>()
            .with::<&Released>()
            .into_iter()
            .collect::<Vec<_>>();
        for entity in &entities_with_released {
            world.remove_one::<Released>(entity.0).unwrap();
        }
    }

    let mut command_buffer = hecs::CommandBuffer::new();

    for (hand_entity, (hand, collider)) in world.query::<(&mut Hand, &Collider)>().iter() {
        // Check to see if we are currently gripping
        if hand.grip_value > 0.1 {
            // If we already have a grabbed entity, no need to do anything.
            if hand.grabbed_entity.is_some() {
                return;
            };

            // Check to see if we are colliding with an entity
            for collided_entity in collider.collisions_this_frame.iter() {
                if world.get::<&Grabbable>(*collided_entity).is_ok() {
                    // If what we're grabbing has a rigid-body, set its body type to kinematic position based so it can be updated with the hand
                    if let Ok(mut rigid_body) = world.get::<&mut RigidBody>(*collided_entity) {
                        rigid_body.body_type = BodyType::KinematicPositionBased;
                    }

                    // If the item we're grabbing has a parent, remove it
                    if world.entity(*collided_entity).unwrap().has::<Parent>() {
                        println!(
                            "Removing parent from grabbed entity: {:?}",
                            *collided_entity
                        );
                        command_buffer.remove_one::<Parent>(*collided_entity);
                    }

                    // Add a "Grabbed" marker trait for other systems to read
                    command_buffer.insert_one(*collided_entity, Grabbed { hand: hand_entity });

                    // Store a reference to the grabbed entity
                    hand.grabbed_entity.replace(*collided_entity);

                    break;
                }
            }
        } else {
            // If we are not gripping, but we have a grabbed entity, release it
            if let Some(grabbed_entity) = hand.grabbed_entity.take() {
                // If what we're grabbing has a rigid-body, set it back to dynamic.
                // TODO: This is a bug. We could have grabbed a rigid-body that was originally kinematic!
                if let Ok(mut rigid_body) = world.get::<&mut RigidBody>(grabbed_entity) {
                    rigid_body.body_type = BodyType::Dynamic;
                }

                // Add a marker trait for other systems to know that this item has at some point been grabbed
                command_buffer.remove_one::<Grabbed>(grabbed_entity);
                command_buffer.insert_one(grabbed_entity, Released);
            }
        }
    }

    command_buffer.run_on(world);
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::components::{hand::Handedness, Info};

    #[test]
    fn test_grabbing_system() {
        let mut world = World::default();

        let grabbed_collider = Collider::default();
        let grabbed_rigid_body = RigidBody::default();
        let grabbed_entity = world.spawn((
            Info {
                name: "Test entity".to_string(),
                node_id: 0,
            },
            Grabbable {},
        ));
        world
            .insert(grabbed_entity, (grabbed_collider, grabbed_rigid_body))
            .unwrap();

        // Fully gripped hand
        let hand = Hand {
            handedness: Handedness::Left,
            grip_value: 1.0,
            grabbed_entity: None,
        };

        // Collider
        let collider = Collider {
            collisions_this_frame: [grabbed_entity].into(),
            ..Default::default()
        };

        let hand_entity = world.spawn((hand, collider));

        tick(&mut world);

        let mut hand = world.get::<&mut Hand>(hand_entity).unwrap();
        assert_eq!(hand.grabbed_entity.unwrap(), grabbed_entity);
        assert_eq!(
            world.get::<&Grabbed>(grabbed_entity).unwrap().hand,
            hand_entity
        );

        hand.grip_value = 0.0;
        drop(hand);

        tick(&mut world);

        let mut hand = world.get::<&mut Hand>(hand_entity).unwrap();
        assert!(hand.grabbed_entity.is_none());

        // Make sure hand can't grip colliders *without* a Grabbable component
        hand.grip_value = 1.0;
        drop(hand);
        world.remove::<(Grabbable,)>(grabbed_entity).unwrap();

        tick(&mut world);

        let hand = world.get::<&mut Hand>(hand_entity).unwrap();
        assert!(hand.grabbed_entity.is_none());
    }

    fn tick(world: &mut World) {
        grabbing_system_inner(world);
    }
}
