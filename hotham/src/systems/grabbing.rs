use glam::Vec3;
use hecs::World;

use crate::{
    components::{
        hand::GrabbedEntity, physics::BodyType, Collider, Grabbable, Grabbed, Hand, LocalTransform,
        Parent, Released, RigidBody,
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

    for (_, (hand, collider, local_transform)) in world
        .query::<(&mut Hand, &Collider, &LocalTransform)>()
        .iter()
    {
        // Check to see if we are currently gripping
        if hand.grip_value > 0.1 {
            // Only grip when button is just pressed
            if !hand.grip_button_just_pressed {
                continue;
            }
            let global_from_grip = local_transform.to_affine();
            let grip_origin_in_global = global_from_grip.transform_point3(Vec3::ZERO);

            // Check to see if we are colliding with an entity
            // Pick the entity closest to the grip origin
            let mut closest_length_squared = f32::INFINITY;
            let mut closest_grippable = None;
            for collided_entity in collider.collisions_this_frame.iter() {
                if world.get::<&Grabbable>(*collided_entity).is_ok() {
                    let global_from_local = world
                        .get::<&LocalTransform>(*collided_entity)
                        .unwrap()
                        .to_affine();
                    let local_origin_in_global = global_from_local.transform_point3(Vec3::ZERO);
                    let length_squared =
                        (local_origin_in_global - grip_origin_in_global).length_squared();
                    if length_squared < closest_length_squared {
                        closest_length_squared = length_squared;
                        closest_grippable = Some(collided_entity);
                    }
                }
            }
            if let Some(entity) = closest_grippable {
                // If the item we're grabbing has a parent, remove it
                if world.entity(*entity).unwrap().has::<Parent>() {
                    println!("Removing parent from grabbed entity: {:?}", *entity);
                    command_buffer.remove_one::<Parent>(*entity);
                }

                // Add a "Grabbed" marker trait for other systems to read
                command_buffer.insert_one(*entity, Grabbed);

                // Store a reference to the grabbed entity
                let global_from_local = world.get::<&LocalTransform>(*entity).unwrap().to_affine();
                let grip_from_local = global_from_grip.inverse() * global_from_local;
                let grabbed_entity = GrabbedEntity {
                    entity: *entity,
                    grip_from_local,
                };
                hand.grabbed_entity.replace(grabbed_entity);
            }
        } else {
            // If we are not gripping, but we have a grabbed entity, release it
            if let Some(grabbed_entity) = hand.grabbed_entity.take() {
                // If what we're grabbing has a rigid-body, set it back to dynamic.
                // TODO: This is a bug. We could have grabbed a rigid-body that was originally kinematic!
                if let Ok(mut rigid_body) = world.get::<&mut RigidBody>(grabbed_entity.entity) {
                    rigid_body.body_type = BodyType::Dynamic;
                }

                // Add a marker trait for other systems to know that this item has at some point been grabbed
                command_buffer.remove_one::<Grabbed>(grabbed_entity.entity);
                command_buffer.insert_one(grabbed_entity.entity, Released);
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
            LocalTransform::default(),
        ));
        world
            .insert(grabbed_entity, (grabbed_collider, grabbed_rigid_body))
            .unwrap();

        // Fully gripped hand
        let hand = Hand {
            handedness: Handedness::Left,
            grip_value: 1.0,
            grip_button_just_pressed: true,
            grabbed_entity: None,
        };

        // Collider
        let collider = Collider {
            collisions_this_frame: [grabbed_entity].into(),
            ..Default::default()
        };

        // A local transform is needed to determine the relative transform.
        let local_transform = LocalTransform::default();

        let hand_entity = world.spawn((hand, collider, local_transform));

        tick(&mut world);

        let mut hand = world.get::<&mut Hand>(hand_entity).unwrap();
        assert_eq!(hand.grabbed_entity.as_ref().unwrap().entity, grabbed_entity);
        hand.grip_value = 0.0;
        hand.grip_button_just_pressed = false;
        drop(hand);

        tick(&mut world);

        let mut hand = world.get::<&mut Hand>(hand_entity).unwrap();
        assert!(hand.grabbed_entity.is_none());

        // Make sure hand can't grip colliders *without* a Grabbable component
        hand.grip_value = 1.0;
        hand.grip_button_just_pressed = true;
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
