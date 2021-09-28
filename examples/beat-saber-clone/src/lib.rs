use hotham::{App, HothamResult};

// use hotham::legion::IntoQuery;
use hotham::{
    add_model_to_world,
    components::{AnimationController, Hand, Transform},
    legion::{IntoQuery, Resources, World},
    rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder},
    resources::PhysicsContext,
    HothamResult as Result, Program,
};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct BeatSaberExample {}

impl BeatSaberExample {
    pub fn new() -> Self {
        Self {}
    }
}

impl Program for BeatSaberExample {
    // TODO: Make more ergonomic
    fn init(
        &mut self,
        models: HashMap<String, World>,
        _resources: &mut Resources,
    ) -> Result<World> {
        let mut world = World::default();
        add_model_to_world("Blue Cube", &models, &mut world, None)
            .expect("Unable to add Blue Cube");
        add_model_to_world("Red Cube", &models, &mut world, None).expect("Unable to add Red Cube");
        add_model_to_world("Blue Saber", &models, &mut world, None)
            .expect("Unable to add Blue Saber");
        add_model_to_world("Red Saber", &models, &mut world, None)
            .expect("Unable to add Red Saber");
        add_model_to_world("Environment", &models, &mut world, None)
            .expect("Unable to add Environment");
        add_model_to_world("Ramp", &models, &mut world, None).expect("Unable to add Ramp");

        Ok(world)
    }

    fn get_gltf_data(&self) -> Vec<&[u8]> {
        vec![include_bytes!("../assets/beat_saber.glb")]
    }
}

#[cfg_attr(target_os = "android", ndk_glue::main(backtrace = "on"))]
pub fn main() {
    println!("[BEAT_SABER_EXAMPLE] MAIN!");
    real_main().unwrap();
}

pub fn real_main() -> HothamResult<()> {
    let program = BeatSaberExample::new();
    let mut app = App::new(program)?;
    app.run()?;
    Ok(())
}
