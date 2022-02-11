use crate::{
    components::{hand::Handedness, AnimationController, Hand, RigidBody},
    gltf_loader::add_model_to_world,
    resources::{PhysicsContext, RenderContext, VulkanContext, XrContext},
    util::{is_space_valid, posef_to_isometry},
};
use hecs::{PreparedQuery, World};
use rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder};

pub fn hands_system(
    query: &mut PreparedQuery<(&mut Hand, &mut AnimationController, &mut RigidBody)>,
    world: &mut World,
    xr_context: &XrContext,
    physics_context: &mut PhysicsContext,
) {
    for (_, (hand, animation_controller, rigid_body_component)) in query.query(world).iter() {
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
        let space = space.locate(&xr_context.reference_space, time).unwrap();

        // Check it's valid before using it
        if !is_space_valid(&space) {
            return;
        }

        let pose = space.pose;

        // apply transform
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body_component.handle)
            .unwrap();

        let position = posef_to_isometry(pose);
        rigid_body.set_next_kinematic_position(position);

        if let Some(grabbed_entity) = hand.grabbed_entity {
            let handle = world.get::<RigidBody>(grabbed_entity).unwrap().handle;
            let rigid_body = physics_context.rigid_bodies.get_mut(handle).unwrap();
            rigid_body.set_next_kinematic_position(position);
        }

        // get grip value
        let grip_value =
            openxr::ActionInput::get(&xr_context.grab_action, &xr_context.session, path)
                .unwrap()
                .current_state;

        // Apply to Hand
        hand.grip_value = grip_value;

        // Apply to AnimationController
        animation_controller.blend_amount = grip_value;
    }
}

pub fn add_hand(
    models: &std::collections::HashMap<String, World>,
    handedness: Handedness,
    world: &mut World,
    vulkan_context: &VulkanContext,
    render_context: &RenderContext,
    physics_context: &mut PhysicsContext,
) {
    let model_name = match handedness {
        Handedness::Left => "Left Hand",
        Handedness::Right => "Right Hand",
    };
    let hand = add_model_to_world(
        model_name,
        models,
        world,
        None,
        vulkan_context,
        &render_context.descriptor_set_layouts,
    )
    .unwrap();
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
            .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
            .build();
        let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
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
        components::Transform, resources::XrContext, systems::update_rigid_body_transforms_system,
    };

    #[test]
    pub fn test_hands_system() {
        let (mut world, mut xr_context, mut physics_context) = setup();
        let hand = add_hand_to_world(&mut physics_context, &mut world, None);

        schedule(&mut world, &mut xr_context, &mut physics_context);

        let (transform, hand, animation_controller) = world
            .query_one_mut::<(&Transform, &Hand, &AnimationController)>(hand)
            .unwrap();

        assert_relative_eq!(hand.grip_value, 0.0);
        assert_relative_eq!(transform.translation, vector![-0.2, 1.4, -0.5]);
        assert_relative_eq!(animation_controller.blend_amount, 0.0);
    }

    #[test]
    pub fn test_move_grabbed_objects() {
        let (mut world, mut xr_context, mut physics_context) = setup();

        let grabbed_object_rigid_body = RigidBodyBuilder::new_kinematic_position_based().build(); // grabber sets the rigidbody as kinematic
        let handle = physics_context
            .rigid_bodies
            .insert(grabbed_object_rigid_body);
        let grabbed_entity = world.spawn((RigidBody { handle }, Transform::default()));
        add_hand_to_world(&mut physics_context, &mut world, Some(grabbed_entity));

        schedule(&mut world, &mut xr_context, &mut physics_context);

        let transform = world.get_mut::<Transform>(grabbed_entity).unwrap();
        assert_relative_eq!(transform.translation, vector![-0.2, 1.4, -0.5]);
    }

    // HELPER FUNCTIONS
    fn setup() -> (World, XrContext, PhysicsContext) {
        let world = World::new();
        let (xr_context, _) = XrContext::new().unwrap();
        let physics_context = PhysicsContext::default();
        (world, xr_context, physics_context)
    }

    fn schedule(
        world: &mut World,
        xr_context: &mut XrContext,
        physics_context: &mut PhysicsContext,
    ) {
        hands_system(&mut Default::default(), world, xr_context, physics_context);
        physics_context.update();
        update_rigid_body_transforms_system(&mut Default::default(), world, physics_context);
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
        let hand = world.spawn((animation_controller, hand, Transform::default()));
        {
            // Give it a collider and rigid-body
            let collider = ColliderBuilder::capsule_y(0.05, 0.02)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::INTERSECTION_EVENTS)
                .build();
            let mut rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
            rigid_body.set_next_kinematic_translation(vector![0.0, 1.4, 0.0]);
            let components =
                physics_context.get_rigid_body_and_collider(hand, rigid_body, collider);
            world.insert(hand, components).unwrap();
        }

        hand
    }
}
