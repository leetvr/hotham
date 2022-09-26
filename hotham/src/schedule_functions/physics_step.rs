use crate::contexts::PhysicsContext;

// TODO: We may want to adjust this so that the tick time is synced with OpenXR
pub fn physics_step(physics_context: &mut PhysicsContext) {
    physics_context.update();
}
