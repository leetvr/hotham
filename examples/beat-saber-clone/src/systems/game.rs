use hotham::components::{Collider, Transform, Visible};
use legion::{system, systems::CommandBuffer};

use crate::components::{Colour, Cube};

#[system(for_each)]
#[read_component(Cube)]
#[read_component(Colour)]
pub fn game(
    command_buffer: &mut CommandBuffer,
    cube: &Cube,
    colour: &Colour,
    transform: &Transform,
    _visible: &Visible,
    collider: &mut Collider,
) {
    // Check if hit
    if let Some(entity) = collider.collisions_this_frame.pop() {}

    // Check if moved too far back
}

#[cfg(test)]
mod tests {
    use hotham::resources::PhysicsContext;
    use legion::World;

    use crate::components::Saber;

    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        // Set up a red cube that has been hit by the red saber
        // Assert score is 1
        let red_saber = world.push((Saber {}, Colour::Red));
        let red_cube = world.push((Cube {}, Colour::Red));

        // Set up a red cube that has bee missed
        // Assert score is 0

        // Set up a blue cube that has been hit by the blue saber
        // Assert score is 1

        // Set up a red cube that has been hit by the red saber
        // Assert score is 2

        // Set up a blue cube that has been hit by the red saber
        // Assert score is 0

        // Set up a blue cube that has been missed
        // Assert score is 0

        // Set up a red cube thas has been missed
        // Assert score is -1.
    }
}
