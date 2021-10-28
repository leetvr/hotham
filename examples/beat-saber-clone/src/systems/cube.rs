use hotham::{components::Transform, gltf_loader::add_model_to_world};
use legion::{system, world::SubWorld, Entity, EntityStore};

use crate::{components::Cube, Models};

#[system]
#[write_component(Transform)]
pub fn cubes(
    world: &mut SubWorld,
    #[state] red_cube_pool: &mut Vec<Entity>,
    #[state] blue_cube_pool: &mut Vec<Entity>,
    #[state] frames_since_last_cube: &mut usize,
) {
    if *frames_since_last_cube == 50 {
        let entity = red_cube_pool.pop().unwrap();
        let mut entry = world.entry_mut(entity).unwrap();
    }
}

mod tests {
    use hotham::components::Transform;
    use legion::{IntoQuery, Resources, Schedule, World};

    use crate::components::Cube;

    use super::*;

    #[test]
    pub fn test_cube() {
        let mut world = World::default();
        let red_cubes = (0..10)
            .map(|_| world.push((Transform::default(),)))
            .collect::<Vec<_>>();
        let blue_cubes = (0..10)
            .map(|_| world.push((Transform::default(),)))
            .collect::<Vec<_>>();

        let mut resources = Resources::default();
        let mut schedule = Schedule::builder()
            .add_system(cubes_system(red_cubes, blue_cubes, 50))
            .build();

        schedule.execute(&mut world, &mut resources);
        let mut query = <&Cube>::query();
        let transforms = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(transforms.len(), 20);
    }
}
