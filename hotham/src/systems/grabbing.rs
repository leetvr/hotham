use hecs::World;

use crate::{
    components::{physics::BodyType, Collider, Grabbable, Hand, RigidBody},
    Engine,
};

/// Grabbing system
/// Used to allow a player to grab objects. Used in conjunction with `hands_system`
pub fn grabbing_system(engine: &mut Engine) {
    let world = &mut engine.world;
    grabbing_system_inner(world);
}

fn grabbing_system_inner(world: &mut World) {
    for (_, (hand, collider)) in world.query::<(&mut Hand, &Collider)>().iter() {
        // Check to see if we are currently gripping
        if hand.grip_value >= 1.0 {
            // If we already have a grabbed entity, no need to do anything.
            if hand.grabbed_entity.is_some() {
                return;
            };

            // Check to see if we are colliding with an entity
            for other_entity in collider.collisions_this_frame.iter() {
                if world.get::<&Grabbable>(*other_entity).is_ok() {
                    // If what we're grabbing has a rigid-body, set its body type to kinematic position based so it can be updated with the hand
                    if let Ok(mut rigid_body) = world.get::<&mut RigidBody>(*other_entity) {
                        rigid_body.body_type = BodyType::KinematicPositionBased;
                    }

                    // Store a reference to the grabbed entity
                    hand.grabbed_entity.replace(*other_entity);

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
            }
        }
    }
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
