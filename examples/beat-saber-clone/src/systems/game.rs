use hotham::components::{Collider, Transform, Visible};
use legion::{system, systems::CommandBuffer, world::SubWorld, Entity, EntityStore};

use crate::{
    components::{Colour, Cube},
    resources::GameState,
};

#[system(for_each)]
#[read_component(&Colour)]
pub fn game(
    entity: &Entity,
    command_buffer: &mut CommandBuffer,
    world: &mut SubWorld,
    _cube: &Cube,
    colour: &Colour,
    transform: &Transform,
    _visible: &Visible,
    collider: &mut Collider,
    #[resource] game_state: &mut GameState,
) {
    // If score is zero, the game is over. Do nothing.
    if game_state.current_score == 0 {
        return;
    }

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
    if let Some(saber) = collider.collisions_this_frame.pop() {
        let saber_entry = world.entry_ref(saber).unwrap();
        let saber_colour = saber_entry.get_component::<Colour>().unwrap();

        // Was this hit with the right colour saber?
        if colour == saber_colour {
            game_state.current_score += 1;
            println!(
                "Entity {:?} was hit! Score is now {}. Removing entity",
                entity, game_state.current_score
            );
        } else {
            game_state.current_score -= 1;
            println!(
                "Entity {:?} was hit with the wrong saber! Score is now {}. Removing entity",
                entity, game_state.current_score
            );
        }

        command_buffer.remove(*entity);
        return;
    }
}

#[cfg(test)]
mod tests {
    use hotham::resources::PhysicsContext;
    use legion::Schedule;
    use legion::{Resources, World};
    use nalgebra::{vector, UnitQuaternion};

    use crate::systems::cube_spawner::add_cube_physics;
    use crate::{components::Saber, systems::sabers::add_saber_physics};

    use super::*;
    #[test]
    pub fn game_system_test() {
        // Setup world.
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();
        let mut resources = Resources::default();

        // Set up our sabers
        let red_saber = world.push((Saber {}, Colour::Red));
        add_saber_physics(&mut world, &mut physics_context, red_saber);

        let blue_saber = world.push((Saber {}, Colour::Blue));
        add_saber_physics(&mut world, &mut physics_context, blue_saber);

        // Add a red cube that will be hit
        let hit_red_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, hit_red_cube);

        // Cube should be present.
        assert!(world.entry(hit_red_cube).is_some());

        // Setup schedule
        resources.insert(GameState::default());
        let mut schedule = Schedule::builder().add_system(game_system()).build();

        // Have the cube be hit by the red saber:
        {
            let mut entry = world.entry(hit_red_cube).unwrap();
            let collider = entry.get_component_mut::<Collider>().unwrap();
            collider.collisions_this_frame.push(red_saber.clone());
            drop(entry);
            schedule.execute(&mut world, &mut resources);

            // Score should be updated
            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 2);

            // Cube should be removed.
            assert!(world.entry(hit_red_cube).is_none());
        }

        // Set up a red cube that has been missed
        // Assert score is 1
        let missed_transform = Transform {
            translation: vector![0., 0., 0.5],
            rotation: UnitQuaternion::identity(),
            scale: vector![1.0, 1.0, 1.0],
        };
        let missed_red_cube = world.push((missed_transform, Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, missed_red_cube);

        schedule.execute(&mut world, &mut resources);
        {
            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 1);
        }

        // Set up a blue cube that has been hit by the blue saber, assert score is now 2.
        let hit_blue_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Blue));
        add_cube_physics(&mut world, &mut physics_context, hit_blue_cube);
        {
            let mut entry = world.entry(hit_blue_cube).unwrap();
            let collider = entry.get_component_mut::<Collider>().unwrap();
            collider.collisions_this_frame.push(blue_saber.clone());
            drop(entry);
            schedule.execute(&mut world, &mut resources);

            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 2);

            // Cube should be removed.
            assert!(world.entry(hit_blue_cube).is_none());
        }

        // Set up a blue cube that has been hit by the red saber, assert score is now 1
        let hit_blue_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Blue));
        add_cube_physics(&mut world, &mut physics_context, hit_blue_cube);
        {
            let mut entry = world.entry(hit_blue_cube).unwrap();
            let collider = entry.get_component_mut::<Collider>().unwrap();
            collider.collisions_this_frame.push(red_saber.clone());
            drop(entry);
            schedule.execute(&mut world, &mut resources);

            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 1);

            // Cube should be removed.
            assert!(world.entry(hit_blue_cube).is_none());
        }

        // Set up a red cube that has been hit by the blue saber, assert score is now 0
        let hit_red_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, hit_red_cube);
        {
            let mut entry = world.entry(hit_red_cube).unwrap();
            let collider = entry.get_component_mut::<Collider>().unwrap();
            collider.collisions_this_frame.push(blue_saber.clone());
            drop(entry);
            schedule.execute(&mut world, &mut resources);

            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 0);

            // Cube should be removed.
            assert!(world.entry(hit_red_cube).is_none());
        }

        // Do the same thing again. Score should stay 0 as we're at game over.
        let hit_red_cube = world.push((Transform::default(), Visible {}, Cube {}, Colour::Red));
        add_cube_physics(&mut world, &mut physics_context, hit_red_cube);
        {
            let mut entry = world.entry(hit_red_cube).unwrap();
            let collider = entry.get_component_mut::<Collider>().unwrap();
            collider.collisions_this_frame.push(blue_saber.clone());
            drop(entry);
            schedule.execute(&mut world, &mut resources);

            let game_state = resources.get::<GameState>().unwrap();
            assert_eq!(game_state.current_score, 0);

            // Cube should _NOT_ be removed.
            assert!(world.entry(hit_red_cube).is_some());
        }
    }
}
