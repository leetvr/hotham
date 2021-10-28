use hotham::{
    components::{Mesh, Transform},
    gltf_loader::add_model_to_world,
};
use legion::{system, world::SubWorld, Entity, EntityStore};

use crate::{components::Cube, Models};

#[system]
#[write_component(Mesh)]
pub fn cube_spawner(
    world: &mut SubWorld,
    #[state] red_cube_pool: &mut Vec<Entity>,
    #[state] blue_cube_pool: &mut Vec<Entity>,
    #[state] frames_since_last_cube: &mut usize,
) {
    if *frames_since_last_cube == 50 {
        let entity = red_cube_pool.pop().unwrap();
        let mut entry = world.entry_mut(entity).unwrap();
        (*entry.get_component_mut::<Mesh>().unwrap()).should_render = true;
    }

    if *frames_since_last_cube >= 100 {
        let entity = blue_cube_pool.pop().unwrap();
        let mut entry = world.entry_mut(entity).unwrap();
        (*entry.get_component_mut::<Mesh>().unwrap()).should_render = true;
    }

    // Reset counter
    if *frames_since_last_cube >= 100 {
        *frames_since_last_cube = 0;
    } else {
        *frames_since_last_cube = *frames_since_last_cube + 1;
    }
}

#[cfg(test)]
mod tests {
    use hotham::{components::Mesh, util::test_mesh};
    use legion::{IntoQuery, Resources, Schedule, World};

    use crate::components::Cube;

    use super::*;

    #[test]
    pub fn test_cube_spawner() {
        let mut world = World::default();
        let red_cubes = (0..10)
            .map(|_| world.push((test_mesh(),)))
            .collect::<Vec<_>>();

        let blue_cubes = (0..10)
            .map(|_| world.push((test_mesh(),)))
            .collect::<Vec<_>>();

        let mut resources = Resources::default();
        let mut schedule = Schedule::builder()
            .add_system(cube_spawner_system(red_cubes, blue_cubes, 50))
            .build();

        schedule.execute(&mut world, &mut resources);
        let mut query = <&Mesh>::query();
        let renderable = query
            .iter(&world)
            .filter(|m| m.should_render)
            .collect::<Vec<_>>();
        assert_eq!(renderable.len(), 1);

        for _ in 0..50 {
            schedule.execute(&mut world, &mut resources);
        }

        let mut query = <&Mesh>::query();
        let renderable = query
            .iter(&world)
            .filter(|m| m.should_render)
            .collect::<Vec<_>>();
        assert_eq!(renderable.len(), 2);
    }
}
