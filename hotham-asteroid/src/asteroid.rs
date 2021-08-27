use std::collections::HashMap;

use cgmath::vec3;
use hotham::{
    add_model_to_world,
    components::{AnimationController, Hand, Transform},
    legion::{Resources, World},
    rapier3d::{
        na as nalgebra,
        na::vector,
        prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    },
    resources::PhysicsContext,
    HothamResult as Result, Program,
};

#[derive(Debug, Clone)]
pub struct Asteroid {}

impl Asteroid {
    pub fn new() -> Self {
        Self {}
    }
}

impl Program for Asteroid {
    // TODO: Make more ergonomic
    fn init(&mut self, models: HashMap<String, World>, resources: &mut Resources) -> Result<World> {
        let mut world = World::default();
        let mut physics_context = resources.get_mut::<PhysicsContext>().unwrap();

        // Add the asteroid model
        let asteroid = add_model_to_world("Asteroid", &models, &mut world, None).unwrap();

        {
            let mut asteroid_entry = world.entry(asteroid).unwrap();
            let asteroid_transform = asteroid_entry.get_component_mut::<Transform>().unwrap();
            asteroid_transform.scale = vec3(0.1, 0.1, 0.1);

            // Give it a collider and rigid-body
            let collider = ColliderBuilder::ball(0.25)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .build();
            let rigid_body = RigidBodyBuilder::new_dynamic()
                .translation(vector![0.0, 1.0, 0.0])
                .build();
            let (collider, rigid_body) =
                physics_context.add_rigid_body_and_collider(asteroid, rigid_body, collider);
            asteroid_entry.add_component(collider);
            asteroid_entry.add_component(rigid_body);
        }

        // Add the refinery model
        add_model_to_world("Refinery", &models, &mut world, Some(asteroid));

        // Add the left hand
        let left_hand = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
        {
            let mut left_hand_entry = world.entry(left_hand).unwrap();

            // Add a hand component
            left_hand_entry.add_component(Hand::left());

            // Modify the animation controller
            let animation_controller = left_hand_entry
                .get_component_mut::<AnimationController>()
                .unwrap();
            animation_controller.blend_from = 0;
            animation_controller.blend_to = 1;

            // Give it a collider and rigid-body
            let collider = ColliderBuilder::capsule_y(0.05, 0.02)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .build();
            let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
            let (collider, rigid_body) =
                physics_context.add_rigid_body_and_collider(left_hand, rigid_body, collider);
            left_hand_entry.add_component(collider);
            left_hand_entry.add_component(rigid_body);
        }

        // Add the right hand
        let right_hand = add_model_to_world("Right Hand", &models, &mut world, None).unwrap();
        {
            let mut right_hand_entry = world.entry(right_hand).unwrap();
            right_hand_entry.add_component(Hand::right());
            let animation_controller = right_hand_entry
                .get_component_mut::<AnimationController>()
                .unwrap();
            animation_controller.blend_from = 0;
            animation_controller.blend_to = 1;

            // Give it a collider and rigid-body
            let collider = ColliderBuilder::capsule_y(0.05, 0.02)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .build();
            let rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
            let (collider, rigid_body) =
                physics_context.add_rigid_body_and_collider(right_hand, rigid_body, collider);
            right_hand_entry.add_component(collider);
            right_hand_entry.add_component(rigid_body);
        }

        // let hello = load_sound("hello.ogg")?;
        // let background = load_sound("background.mp3")?;
        // let sounds = vec![hello, background];
        Ok(world)
    }

    fn get_gltf_data(&self) -> Vec<(&[u8], &[u8])> {
        vec![
            (
                include_bytes!("../assets/asteroid.gltf"),
                include_bytes!("../assets/asteroid_data.bin"),
            ),
            (
                include_bytes!("../assets/left_hand.gltf"),
                include_bytes!("../assets/left_hand.bin"),
            ),
            (
                include_bytes!("../assets/right_hand.gltf"),
                include_bytes!("../assets/right_hand.bin"),
            ),
        ]
    }
}
