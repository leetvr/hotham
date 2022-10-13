use crate::{
    asset_importer::add_model_to_world,
    components::{
        global_transform::GlobalTransform, hand::Handedness, local_transform::LocalTransform,
        stage, AnimationController, Collider, Hand,
    },
    contexts::{physics_context::HAND_COLLISION_GROUP, InputContext, PhysicsContext},
    Engine,
};
use hecs::World;
use rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, InteractionGroups};

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
            let mut local_transform = world.get::<&mut LocalTransform>(grabbed_entity).unwrap();
            local_transform.update_from_affine(&global_from_local);
            let mut global_transform = world.get::<&mut GlobalTransform>(grabbed_entity).unwrap();
            global_transform.0 = local_transform.to_affine();
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
    physics_context: &mut PhysicsContext,
) {
    let (hand_component, model_name) = match handedness {
        Handedness::Left => (Hand::left(), "Left Hand"),
        Handedness::Right => (Hand::right(), "Right Hand"),
    };

    // Spawn the hand
    let hand_entity = add_model_to_world(model_name, models, world, physics_context, None).unwrap();

    // Modify the animation controller
    let mut animation_controller = world.get::<&mut AnimationController>(hand_entity).unwrap();
    animation_controller.blend_from = 0;
    animation_controller.blend_to = 1;
    drop(animation_controller);

    // Give it a collider
    let collider = ColliderBuilder::capsule_y(0.05, 0.02)
        .sensor(true)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::COLLISION_EVENTS)
        .collision_groups(InteractionGroups::new(HAND_COLLISION_GROUP, u32::MAX))
        .user_data(hand_entity.to_bits().get() as _)
        .build();

    let collider = Collider::new(physics_context.colliders.insert(collider));

    world
        .insert(hand_entity, (collider, hand_component))
        .unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use hecs::Entity;
    use rapier3d::prelude::{RigidBodyBuilder, RigidBodyType};

    use crate::components::{LocalTransform, RigidBody};

    #[test]
    pub fn test_hands_system() {
        let (mut world, input_context, _) = setup();
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
        let (mut world, input_context, mut physics_context) = setup();

        let grabbed_object_rigid_body =
            RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build(); // grabber sets the rigid body as kinematic
        let handle = physics_context
            .rigid_bodies
            .insert(grabbed_object_rigid_body);
        let grabbed_entity = world.spawn((
            RigidBody { handle },
            LocalTransform::default(),
            GlobalTransform::default(),
        ));
        add_hand_to_world(&mut world, Some(grabbed_entity));

        tick(&mut world, &input_context);

        let local_transform = world.get::<&mut LocalTransform>(grabbed_entity).unwrap();
        assert_relative_eq!(local_transform.translation, [-0.2, 1.4, -0.5].into());
    }

    // HELPER FUNCTIONS
    fn setup() -> (World, InputContext, PhysicsContext) {
        let world = World::new();
        let input_context = InputContext::testing();
        let physics_context = PhysicsContext::default();
        (world, input_context, physics_context)
    }

    fn tick(world: &mut World, input_context: &InputContext) {
        hands_system_inner(world, input_context);
    }

    fn add_hand_to_world(world: &mut World, grabbed_entity: Option<Entity>) -> Entity {
        let mut animation_controller = AnimationController::default();
        animation_controller.blend_amount = 100.0; // bogus value

        let mut hand = Hand::left();
        hand.grip_value = 100.0; // bogus value
        hand.grabbed_entity = grabbed_entity;
        world.spawn((
            animation_controller,
            hand,
            LocalTransform::default(),
            GlobalTransform::default(),
        ))
    }
}
