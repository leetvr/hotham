use hotham::components::Transform;
use legion::{system, world::SubWorld, Entity};

#[system]
#[write_component(Transform)]
pub fn cubes(_world: &mut SubWorld, #[resource] _cube_state: &CubeState) {}

pub struct CubeState {
    pub blue_cube: Entity,
    pub red_cube: Entity,
}

mod tests {
    use hotham::components::Transform;
    use legion::{IntoQuery, Resources, Schedule, World};

    use super::*;

    #[test]
    pub fn test_cube() {
        let mut world = World::default();
        let red_cube = world.push((Transform::default(),));
        let blue_cube = world.push((Transform::default(),));
        let state = CubeState {
            red_cube,
            blue_cube,
        };

        let mut resources = Resources::default();
        resources.insert(state);
        let mut schedule = Schedule::builder().add_system(cubes_system()).build();

        schedule.execute(&mut world, &mut resources);
        let mut query = <&Transform>::query();
        let transforms = query.iter(&world).collect::<Vec<_>>();
        assert_eq!(transforms.len(), 4);
    }
}
