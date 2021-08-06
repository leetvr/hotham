use std::{cell::RefCell, collections::HashMap, rc::Rc};

use cgmath::{vec3, Euler, Quaternion, Rad};
use hotham::{
    load_sound, node::Node, HothamError, HothamResult as Result, Program, ProgramInitialization,
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
    fn init(&mut self, nodes: HashMap<String, Rc<RefCell<Node>>>) -> Result<ProgramInitialization> {
        let asteroid = nodes
            .get("Asteroid")
            .ok_or(HothamError::EmptyListError)?
            .clone();

        let mut asteroid = Node::clone(&asteroid.borrow());
        let translation = vec3(0.0, 1.0, 0.0);
        let rotation = vec3(0.0, 0.0, 0.0);
        let scale = vec3(0.1, 0.1, 0.1);
        asteroid.scale = scale;
        let rotation = Euler::new(Rad(rotation.x), Rad(rotation.y), Rad(rotation.z));
        asteroid.rotation = Quaternion::from(rotation);
        asteroid.translation = translation;

        let asteroid = Rc::new(RefCell::new(asteroid));
        (*asteroid).borrow_mut().children.iter_mut().for_each(|c| {
            (*c).borrow_mut().parent = Rc::downgrade(&asteroid);
        });

        let refinery = nodes
            .get("Refinery")
            .ok_or(HothamError::EmptyListError)?
            .clone();
        let refinery = Node::clone(&refinery.borrow());

        let refinery = Rc::new(RefCell::new(refinery));
        let refinery_parent = Rc::downgrade(&asteroid);
        (*refinery).borrow_mut().children.iter_mut().for_each(|c| {
            (*c).borrow_mut().parent = Rc::downgrade(&refinery);
        });
        (*refinery).borrow_mut().parent = refinery_parent;

        let nodes = vec![asteroid, refinery];
        let hello = load_sound("hello.ogg")?;
        let background = load_sound("background.mp3")?;
        let sounds = vec![hello, background];

        Ok(ProgramInitialization { nodes, sounds })
    }

    fn get_gltf_data(&self) -> (&[u8], &[u8]) {
        (
            include_bytes!("../assets/asteroid.gltf"),
            include_bytes!("../assets/asteroid_data.bin"),
        )
    }
}
