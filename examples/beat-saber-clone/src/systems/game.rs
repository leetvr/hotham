use hotham::components::{Collider, Transform, Visible};
use legion::{system, systems::CommandBuffer, Entity};

use crate::{
    components::{Colour, Cube},
    resources::GameState,
};

#[system(for_each)]
pub fn game(
    entity: &Entity,
    command_buffer: &mut CommandBuffer,
    _cube: &Cube,
    _colour: &Colour,
    transform: &Transform,
    _visible: &Visible,
    collider: &mut Collider,
    #[resource] game_state: &mut GameState,
) {
    // Check if moved too far back
    if transform.translation.z > 0.0 {
        game_state.current_score -= 1;
        println!(
            "Entity {:?} was missed! Score is now {}. Removing entity",
            entity, game_state.current_score
        );
        command_buffer.remove(*entity);
        return;
    }

    // Check if hit
    if let Some(_saber) = collider.collisions_this_frame.pop() {
        game_state.current_score += 1;
        println!(
            "Entity {:?} was hit! Score is now {}. Removing entity",
            entity, game_state.current_score
        );
        command_buffer.remove(*entity);
        return;
    }
}

#[cfg(test)]
mod tests {
    use hotham::resources::PhysicsContext;
    use hotham::schedule_functions::physics_step;
    use hotham::systems::collision_system;
    use legion::{component, IntoQuery, Schedule};
    use legion::{Resources, World};
    use nalgebra::{vector, UnitQuaternion};

    use crate::systems::cube_spawner::add_cube_physics;
    use crate::{components::Saber, systems::sabers::add_saber_physics};

    use super::*;
    #[test]
    pub fn game_system_test() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        // Set up a red cube that has been hit by the red saber
        let red_saber = world.push((Saber {}, Colour::Red));
        add_saber_physics(&mut world, &mut physics_context, red_saber);

        let hit_red_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, hit_red_cube);

        // Cube should be present.
        assert!(world.entry(hit_red_cube).is_some());

        let mut resources = Resources::default();
        resources.insert(physics_context);
        resources.insert(GameState::default());
        let mut schedule = Schedule::builder()
            .add_thread_local_fn(physics_step)
            .add_system(collision_system())
            .add_system(game_system())
            .build();
        schedule.execute(&mut world, &mut resources);

        {
            // Score should be updated
            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 2);

            // Cube should be removed.
            assert!(world.entry(hit_red_cube).is_none());
        }

        // Set up a red cube that has been missed
        // Assert score is 1
        let mut physics_context = resources.get_mut::<PhysicsContext>().unwrap();
        let missed_transform = Transform {
            translation: vector![0., 0., 0.5],
            rotation: UnitQuaternion::identity(),
            scale: vector![1.0, 1.0, 1.0],
        };
        let missed_red_cube = world.push((missed_transform, Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, missed_red_cube);
        drop(physics_context);

        schedule.execute(&mut world, &mut resources);
        {
            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 1);
        }

        // Set up a blue cube that has been hit by the blue saber
        // Assert score is 2

        // Set up a red cube that has been hit by the red saber
        // Assert score is 3

        // Set up a blue cube that has been hit by the red saber
        // Assert score is 2

        // Set up a blue cube that has been missed
        // Assert score is 1

        // Set up a red cube thas has been missed
        // Assert score is 0.
    }
}
