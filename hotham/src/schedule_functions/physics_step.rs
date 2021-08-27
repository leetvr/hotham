use legion::{Resources, World};

use crate::resources::PhysicsContext;

// TODO: We may want to adjust this so that the tick time is synced with OpenXR
pub fn physics_step(_: &mut World, resources: &mut Resources) -> () {
    resources.get_mut::<PhysicsContext>().unwrap().update();
}
