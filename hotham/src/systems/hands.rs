use crate::{
    asset_importer::add_model_to_world,
    components::{
        global_transform::GlobalTransform, hand::Handedness, local_transform::LocalTransform,
        stage, AnimationController, Collider, Grabbed, Hand,
    },
    contexts::{physics_context::HAND_COLLISION_GROUP, InputContext},
    Engine,
};
use hecs::World;
use rapier3d::prelude::{ActiveCollisionTypes, Group, SharedShape};

/// Hands system
/// Used to allow users to interact with objects using their controllers as representations of their hands
pub fn hands_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &mut engine.input_context;
    hands_system_inner(world, input_context);
}

pub fn hands_system_inner(world: &mut World, input_context: &InputContext) {
    // Get the position
    let global_from_stage = stage::get_global_from_stage(world);

    for (_, (hand, animation_controller, local_transform, global_transform)) in world
        .query::<(
            &mut Hand,
            &mut AnimationController,
            &mut LocalTransform,
            &mut GlobalTransform,
        )>()
        .iter()
    {
        // Get the position of the hand in stage space.
        let (stage_from_grip, grip_value) = match hand.handedness {
            Handedness::Left => (
                input_context.left.stage_from_grip(),
                input_context.left.grip_analog(),
            ),
            Handedness::Right => (
                input_context.right.stage_from_grip(),
                input_context.right.grip_analog(),
            ),
        };

        // Get global transform
        let global_from_local = global_from_stage * stage_from_grip;

        // Apply transform
        local_transform.update_from_affine(&global_from_local);
        global_transform.0 = global_from_local;

        // If we've grabbed something, update its transform, being careful to preserve its scale.
        if let Some(grabbed_entity) = hand.grabbed_entity {
            // We first need to check if some other system has decided that this item should no longer be grabbed.
            if !world.entity(grabbed_entity).unwrap().has::<Grabbed>() {
                hand.grabbed_entity = None;
            } else {
                // OK. We are sure that this entity exists, and is being grabbed.
                let mut local_transform = world.get::<&mut LocalTransform>(grabbed_entity).unwrap();
                local_transform.update_rotation_translation_from_affine(&global_from_local);

                let mut global_transform =
                    world.get::<&mut GlobalTransform>(grabbed_entity).unwrap();
                *global_transform = (*local_transform).into();
            }
        }

        // Apply grip value to hand
        hand.grip_value = grip_value;

        // Apply to AnimationController
        animation_controller.blend_amount = grip_value;
    }
}

/// Convenience function to add a Hand, Collider and corresponding Mesh to the world
pub fn add_hand(
    models: &std::collections::HashMap<String, World>,
    handedness: Handedness,
    world: &mut World,
) {
    let (hand_component, model_name) = match handedness {
        Handedness::Left => (Hand::left(), "Left Hand"),
        Handedness::Right => (Hand::right(), "Right Hand"),
    };

    // Spawn the hand
    let hand_entity = add_model_to_world(model_name, models, world, None).unwrap();

    // Modify the animation controller
    let mut animation_controller = world.get::<&mut AnimationController>(hand_entity).unwrap();
    animation_controller.blend_from = 0;
    animation_controller.blend_to = 1;
    drop(animation_controller);

    // Give it a collider
    let collider = Collider {
        shape: SharedShape::capsule_y(0.05, 0.02),
        sensor: true,
        active_collision_types: ActiveCollisionTypes::all(),
        collision_groups: HAND_COLLISION_GROUP,
        collision_filter: Group::all(),
        ..Default::default()
    };

    world
        .insert(hand_entity, (collider, hand_component))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use glam::Vec3;
    use hecs::Entity;

    use crate::components::{LocalTransform, RigidBody};

    #[test]
    pub fn test_hands_system() {
        let (mut world, input_context) = setup();
        let hand = add_hand_to_world(&mut world, None);

        tick(&mut world, &input_context);

        let (local_transform, hand, animation_controller) = world
            .query_one_mut::<(&LocalTransform, &Hand, &AnimationController)>(hand)
            .unwrap();

        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(local_transform.translation, [-0.2, 1.4, -0.5].into());
        assert_relative_eq!(animation_controller.blend_amount, 0.0);
    }

    #[test]
    pub fn test_move_grabbed_objects() {
        let (mut world, input_context) = setup();

        let expected_scale = Vec3::X * 1000.;
        let grabbed_entity = world.spawn((
            Grabbed,
            RigidBody::default(),
            LocalTransform {
                scale: expected_scale,
                ..Default::default()
            },
            GlobalTransform::default(),
        ));
        add_hand_to_world(&mut world, Some(grabbed_entity));

        tick(&mut world, &input_context);

        let local_transform = world.get::<&mut LocalTransform>(grabbed_entity).unwrap();
        assert_relative_eq!(local_transform.translation, [-0.2, 1.4, -0.5].into());

        // Make sure that scale gets preserved
        assert_relative_eq!(local_transform.scale, expected_scale);
    }

    #[test]
    pub fn test_ungrabbed_object_do_not_move() {
        let (mut world, input_context) = setup();

        let grabbed_entity = world.spawn((
            RigidBody::default(),
            LocalTransform::default(),
            GlobalTransform::default(),
        ));
        add_hand_to_world(&mut world, Some(grabbed_entity));

        tick(&mut world, &input_context);

        let local_transform = world.get::<&mut LocalTransform>(grabbed_entity).unwrap();
        assert_relative_eq!(local_transform.translation, Default::default());
        assert_relative_eq!(local_transform.scale, Vec3::ONE);
        assert_relative_eq!(local_transform.rotation, Default::default());
    }

    // HELPER FUNCTIONS
    fn setup() -> (World, InputContext) {
        let world = World::new();
        let input_context = InputContext::testing();
        (world, input_context)
    }

    fn tick(world: &mut World, input_context: &InputContext) {
        hands_system_inner(world, input_context);
    }

    fn add_hand_to_world(world: &mut World, grabbed_entity: Option<Entity>) -> Entity {
        let animation_controller = AnimationController {
            blend_amount: 100.0, // bogus value
            ..Default::default()
        };
        let hand = Hand {
            grip_value: 100.0, // bogus value
            grabbed_entity,
            ..Hand::left()
        };
        world.spawn((
            animation_controller,
            hand,
            LocalTransform::default(),
            GlobalTransform::default(),
        ))
    }
}
