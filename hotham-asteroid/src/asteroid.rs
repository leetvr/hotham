use std::collections::HashMap;

use cgmath::vec3;
use hotham::{
    add_model_to_world,
    components::Transform,
    legion::{IntoQuery, World},
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

        let _refinery =
            add_model_to_world("Refinery", &models, &mut world, Some(asteroid)).unwrap();

        // let hello = load_sound("hello.ogg")?;
        // let background = load_sound("background.mp3")?;
        // let sounds = vec![hello, background];
        Ok(world)
    }

    fn get_gltf_data(&self) -> (&[u8], &[u8]) {
        (
            include_bytes!("../assets/asteroid.gltf"),
            include_bytes!("../assets/asteroid_data.bin"),
        )
    }
}
