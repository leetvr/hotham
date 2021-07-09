use std::collections::HashMap;

use cgmath::{vec3, Euler, Matrix4, Quaternion, Rad};
use hotham::{
    model::{Model, SceneObject},
    HothamError, HothamResult as Result, Program,
};

#[derive(Debug, Clone)]
pub struct Asteroid {}

impl Asteroid {
    pub fn new() -> Self {
        Self {}
    }
}

impl Program for Asteroid {
    fn init(&mut self, models: HashMap<String, Model>) -> Result<Vec<SceneObject>> {
        let asteroid_model = models.get("Asteroid").ok_or(HothamError::EmptyListError)?;
        let translation = vec3(0.0, 1.0, 0.0);
        let rotation = vec3(0.0, 0.0, 0.0);
        let scale = 0.1;

        let scale = Matrix4::from_scale(scale);
        let rotation = Euler::new(Rad(rotation.x), Rad(rotation.y), Rad(rotation.z));
        let rotation = Quaternion::from(rotation);
        let rotation = Matrix4::from(rotation);
        let translation = Matrix4::from_translation(translation);

        let transform = translation * rotation * scale;
        let asteroid = SceneObject::new(asteroid_model.clone(), transform);

        let refinery_model = models.get("Refinery").ok_or(HothamError::EmptyListError)?;
        let refinery = SceneObject::new(
            refinery_model.clone(),
            asteroid.transform * refinery_model.transform,
        );

        Ok(vec![asteroid, refinery])
    }

    fn get_model_data(&self) -> (&[u8], &[u8]) {
        (
            include_bytes!("../assets/asteroid.gltf"),
            include_bytes!("../assets/asteroid_data.bin"),
        )
    }
}
