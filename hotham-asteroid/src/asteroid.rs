use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cgmath::{vec3, Euler, Quaternion, Rad};
use hotham::{load_sound, HothamError, HothamResult as Result, Program, World};

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
        let world = World::default();
        // let asteroid = world
        //     .get("Asteroid")
        //     .ok_or(HothamError::EmptyListError)?
        //     .clone();

        // let mut asteroid = Node::clone(&asteroid.borrow());
        // let translation = vec3(0.0, 1.0, 0.0);
        // let rotation = vec3(0.0, 0.0, 0.0);
        // let scale = vec3(0.1, 0.1, 0.1);
        // asteroid.scale = scale;
        // let rotation = Euler::new(Rad(rotation.x), Rad(rotation.y), Rad(rotation.z));
        // asteroid.rotation = Quaternion::from(rotation);
        // asteroid.translation = translation;

        // let refinery = nodes
        //     .get("Refinery")
        //     .ok_or(HothamError::EmptyListError)?
        //     .clone();
        // let refinery = Node::clone(&refinery.borrow());

        // let nodes = vec![(asteroid), (refinery)];
        // let hello = load_sound("hello.ogg")?;
        // let background = load_sound("background.mp3")?;
        // let sounds = vec![hello, background];

        // let entites = world.extend(nodes);

        Ok(world)
    }

    fn get_gltf_data(&self) -> (&[u8], &[u8]) {
        (
            include_bytes!("../assets/asteroid.gltf"),
            include_bytes!("../assets/asteroid_data.bin"),
        )
    }
}
