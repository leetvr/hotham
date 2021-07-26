use std::{collections::HashMap, rc::Rc};

use cgmath::{vec3, Euler, Quaternion, Rad};
use hotham::{node::Node, HothamError, HothamResult as Result, Program};

#[derive(Debug, Clone)]
pub struct Asteroid {}

impl Asteroid {
    pub fn new() -> Self {
        Self {}
    }
}

impl Program for Asteroid {
    fn init(&mut self, nodes: HashMap<String, Rc<Node>>) -> Result<Vec<Node>> {
        let asteroid = nodes.get("Asteroid").ok_or(HothamError::EmptyListError)?;
        let mut asteroid = Node::clone(asteroid);
        let translation = vec3(0.0, 1.0, 0.0);
        let rotation = vec3(0.0, 0.0, 0.0);
        let scale = vec3(0.1, 0.1, 0.1);

        asteroid.scale = scale;
        let rotation = Euler::new(Rad(rotation.x), Rad(rotation.y), Rad(rotation.z));
        asteroid.rotation = Quaternion::from(rotation);
        asteroid.translation = translation;

        // TODO: asteroid.update_local_matrix();
        let refinery = nodes.get("Refinery").ok_or(HothamError::EmptyListError)?;
        let refinery = Node::clone(refinery);

        Ok(vec![asteroid, refinery])
    }

    fn get_gltf_data(&self) -> (&[u8], &[u8]) {
        (
            include_bytes!("../assets/asteroid.gltf"),
            include_bytes!("../assets/asteroid_data.bin"),
        )
    }
}
