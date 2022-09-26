use crate::Engine;

/// Update the physics simulation.
pub fn physics_system(engine: &mut Engine) {
    // TODO: We may want to adjust this so that the tick time is synced with OpenXR
    engine.physics_context.update();
}
