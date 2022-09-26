use crate::{
    asset_importer::add_model_to_world,
    components::{hand::Handedness, AnimationController, Hand, RigidBody, Stage},
    resources::{physics_context, InputContext, PhysicsContext},
    Engine,
};
use hecs::{PreparedQuery, With, World};
use rapier3d::prelude::{
    ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder, RigidBodyType,
};

/// Hands system
/// Used to allow users to interact with objects using their controllers as representations of their hands
pub fn hands_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let input_context = &mut engine.input_context;
    let physics_context = &mut engine.physics_context;

    hands_system_inner(world, input_context, physics_context);
}

pub fn hands_system_inner(
    world: &mut World,
    input_context: &InputContext,
    physics_context: &mut PhysicsContext,
) {
    // Get the isometry of the stage
    let global_from_stage = world
        .query_mut::<With<Stage, &RigidBody>>()
        .into_iter()
        .next()
        .map_or(Default::default(), |(_, rigid_body)| {
            *physics_context.rigid_bodies[rigid_body.handle].position()
        });

    for (_, (hand, animation_controller, rigid_body_component)) in world
        .query::<(&mut Hand, &mut AnimationController, &mut RigidBody)>()
        .iter()
    {
        // Get our the space and path of the hand.
        let (stage_from_local, grip_value) = match hand.handedness {
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
        let global_from_local = global_from_stage * stage_from_local;

        // Apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body_component.handle)
            .unwrap();

        rigid_body.set_next_kinematic_position(global_from_local);

        if let Some(grabbed_entity) = hand.grabbed_entity {
            let handle = world.get::<RigidBody>(grabbed_entity).unwrap().handle;
            let rigid_body = physics_context.rigid_bodies.get_mut(handle).unwrap();
            rigid_body.set_next_kinematic_position(global_from_local);
        }

        // Apply grip value to hand
        hand.grip_value = grip_value;

        // Apply to AnimationController
        animation_controller.blend_amount = grip_value;
    }
}

/// Convenience function to add a Hand and corresponding Mesh to the world
pub fn add_hand(
    models: &std::collections::HashMap<String, World>,
    handedness: Handedness,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    let model_name = match handedness {
        Handedness::Left => "Left Hand",
        Handedness::Right => "Right Hand",
    };
    let hand = add_model_to_world(model_name, models, world, None).unwrap();
    {
        // Add a hand component
        world
            .insert_one(
                hand,
                Hand {
                    grip_value: 0.,
                    handedness,
                    grabbed_entity: None,
                },
            )
            .unwrap();

        // Modify the animation controller
        let mut animation_controller = world.get_mut::<AnimationController>(hand).unwrap();
        animation_controller.blend_from = 0;
        animation_controller.blend_to = 1;
        drop(animation_controller);

        // Give it a collider and rigid-body
        let collider = ColliderBuilder::capsule_y(0.05, 0.02)
            .sensor(true)
            .active_collision_types(ActiveCollisionTypes::all())
            .active_events(ActiveEvents::COLLISION_EVENTS)
            .build();
        let rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build();
        let components = physics_context.get_rigid_body_and_collider(hand, rigid_body, collider);
        world.insert(hand, components).unwrap();
    }
}

#[cfg(target_os = "windows")]
#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use hecs::Entity;
    use nalgebra::vector;
    use rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder,
    };

    use crate::{
        components::LocalTransform,
        resources::XrContext,
        systems::{
            update_local_transform_with_rigid_body::update_local_transform_with_rigid_body_system_inner,
            update_local_transform_with_rigid_body_system,
        },
    };

    #[test]
    pub fn test_hands_system() {
        let (mut world, mut input_context, xr_context, mut physics_context) = setup();

        input_context.update(&xr_context);

        let hand = add_hand_to_world(&mut physics_context, &mut world, None);

        tick(&mut world, &input_context, &mut physics_context);

        let (local_transform, hand, animation_controller) = world
            .query_one_mut::<(&LocalTransform, &Hand, &AnimationController)>(hand)
            .unwrap();

        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(local_transform.translation, vector![-0.2, 1.4, -0.5]);
        assert_relative_eq!(animation_controller.blend_amount, 0.0);
    }

    #[test]
    pub fn test_move_grabbed_objects() {
        let (mut world, mut input_context, xr_context, mut physics_context) = setup();

        input_context.update(&xr_context);

        let grabbed_object_rigid_body =
            RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build(); // grabber sets the rigid body as kinematic
        let handle = physics_context
            .rigid_bodies
            .insert(grabbed_object_rigid_body);
        let grabbed_entity = world.spawn((RigidBody { handle }, LocalTransform::default()));
        add_hand_to_world(&mut physics_context, &mut world, Some(grabbed_entity));

        tick(&mut world, &input_context, &mut physics_context);

        let local_transform = world.get_mut::<LocalTransform>(grabbed_entity).unwrap();
        assert_relative_eq!(local_transform.translation, vector![-0.2, 1.4, -0.5]);
    }

    // HELPER FUNCTIONS
    fn setup() -> (World, InputContext, XrContext, PhysicsContext) {
        let world = World::new();
        let input_context = InputContext::default();
        let (xr_context, _) = XrContext::testing();
        let physics_context = PhysicsContext::default();
        (world, input_context, xr_context, physics_context)
    }

    fn tick(world: &mut World, input_context: &InputContext, physics_context: &mut PhysicsContext) {
        hands_system_inner(world, input_context, physics_context);
        physics_context.update();
        update_local_transform_with_rigid_body_system_inner(world, physics_context);
    }

    fn add_hand_to_world(
        physics_context: &mut PhysicsContext,
        world: &mut World,
        grabbed_entity: Option<Entity>,
    ) -> Entity {
        let mut animation_controller = AnimationController::default();
        animation_controller.blend_amount = 100.0; // bogus value

        let mut hand = Hand::left();
        hand.grip_value = 100.0; // bogus value
        hand.grabbed_entity = grabbed_entity;
        let hand = world.spawn((animation_controller, hand, LocalTransform::default()));
        {
            // Give it a collider and rigid-body
            let collider = ColliderBuilder::capsule_y(0.05, 0.02)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .build();
            let mut rigid_body =
                RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build();
            rigid_body.set_next_kinematic_translation(vector![0.0, 1.4, 0.0]);
            let components =
                physics_context.get_rigid_body_and_collider(hand, rigid_body, collider);
            world.insert(hand, components).unwrap();
        }

        hand
    }
}
