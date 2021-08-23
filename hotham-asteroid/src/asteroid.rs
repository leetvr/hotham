use std::collections::HashMap;

use cgmath::vec3;
use hotham::{
    add_model_to_world,
    components::{AnimationController, Hand, Info, Transform},
    legion::{EntityStore, IntoQuery, World},
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
    fn init(&mut self, models: HashMap<String, World>) -> Result<World> {
        let mut world = World::default();
        let asteroid = add_model_to_world("Asteroid", &models, &mut world, None).unwrap();
        let mut query = <&mut Transform>::query();
        let asteroid_transform = query.get_mut(&mut world, asteroid).unwrap();
        asteroid_transform.translation = vec3(0.0, 1.0, 0.0);
        asteroid_transform.scale = vec3(0.1, 0.1, 0.1);
        drop(asteroid_transform);

        add_model_to_world("Refinery", &models, &mut world, Some(asteroid));
        let node_name = "Left Hand";

        let root_entity = add_model_to_world("Left Hand", &models, &mut world, None).unwrap();
        {
            let mut query = <&Info>::query();
            let hand = query
                .iter_chunks(&world)
                .map(|chunk| chunk.into_iter_entities())
                .flatten()
                .filter(|(_, info)| info.name == node_name)
                .next()
                .unwrap();
            let (hand_entity, info) = hand;
            println!("Left Hand is {:?}", info);
            let mut hand_entity = world.entry(hand_entity).unwrap();
            hand_entity.add_component(Hand::left());
        }

        let mut root_entry = world.entry(root_entity).unwrap();
        {
            let animation_controller = root_entry
                .get_component_mut::<AnimationController>()
                .unwrap();
            animation_controller.blend_from = 0;
            animation_controller.blend_to = 1;
        }

        let right_hand = add_model_to_world("Right Hand", &models, &mut world, None).unwrap();
        let mut right_hand_entity = world.entry(right_hand).unwrap();
        right_hand_entity.add_component(Hand::right());
        {
            let animation_controller = right_hand_entity
                .get_component_mut::<AnimationController>()
                .unwrap();
            animation_controller.blend_from = 0;
            animation_controller.blend_to = 1;
        }

        let info = world
            .entry_ref(right_hand)
            .unwrap()
            .get_component::<Info>()
            .unwrap()
            .clone();
        println!("Right hand is: {:?}", info);

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
