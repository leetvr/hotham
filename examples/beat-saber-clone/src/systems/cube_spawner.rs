use hotham::{
    components::{Mesh, RigidBody, Transform},
    gltf_loader::add_model_to_world,
    resources::{physics_context, PhysicsContext},
};
use legion::{
    system,
    world::{EntryMut, SubWorld},
    Entity, EntityStore, World,
};
use nalgebra::{vector, Vector3};
use rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder};

use crate::{
    components::{cube, Cube},
    Models,
};

const CUBE_STARTING_VELOCITY: Vector3<f32> = Vector3::new(0., 0., 0.03);
const CUBE_STARTING_DISTANCE: f32 = -1.;
const CUBE_STARTING_HEIGHT: f32 = 1.2;

#[system]
#[write_component(Mesh)]
#[write_component(RigidBody)]
pub fn cube_spawner(
    world: &mut SubWorld,
    #[state] red_cube_pool: &mut Vec<Entity>,
    #[state] blue_cube_pool: &mut Vec<Entity>,
    #[state] frames_since_last_cube: &mut usize,
    #[resource] mut physics_context: &mut PhysicsContext,
) {
    if red_cube_pool.len() == 0 || blue_cube_pool.len() == 0 {
        return;
    }

    if *frames_since_last_cube == 50 {
        let entity = red_cube_pool.pop().unwrap();
        let mut entry = world.entry_mut(entity).unwrap();
        activate_cube(&mut entry, &mut physics_context);
    }

    if *frames_since_last_cube == 100 {
        let entity = blue_cube_pool.pop().unwrap();
        let mut entry = world.entry_mut(entity).unwrap();
        activate_cube(&mut entry, &mut physics_context);
    }

    // Reset counter
    if *frames_since_last_cube >= 200 {
        *frames_since_last_cube = 0;
    } else {
        *frames_since_last_cube = *frames_since_last_cube + 1;
    }
}

fn activate_cube(cube: &mut EntryMut, physics_context: &mut PhysicsContext) {
    let mut mesh = cube.get_component_mut::<Mesh>().unwrap();
    mesh.should_render = true;
    let r = cube.get_component::<RigidBody>().unwrap();

    println!("Activating cube: {:?}", r.handle);
    let rigid_body = &mut physics_context.rigid_bodies[r.handle];
    rigid_body.set_linvel(CUBE_STARTING_VELOCITY, true);
    rigid_body.set_translation(
        vector!(0., CUBE_STARTING_HEIGHT, CUBE_STARTING_DISTANCE),
        true,
    );
}

pub fn add_cube_physics(world: &mut World, physics_context: &mut PhysicsContext, cube: Entity) {
    let mut cube_entry = world.entry(cube).unwrap();

    // Give it a collider and rigid-body
    let collider = ColliderBuilder::cuboid(0.1, 0.1, 0.1)
        .active_collision_types(ActiveCollisionTypes::all())
        .active_events(ActiveEvents::INTERSECTION_EVENTS)
        .build();
    let rigid_body = RigidBodyBuilder::new_dynamic().lock_rotations().build();
    let (collider, rigid_body) =
        physics_context.add_rigid_body_and_collider(cube, rigid_body, collider);
    cube_entry.add_component(collider);
    cube_entry.add_component(rigid_body);
}

pub fn create_cubes(
    count: usize,
    colour: cube::Colour,
    models: &Models,
    mut world: &mut World,
    mut physics_context: &mut PhysicsContext,
) -> Vec<Entity> {
    let model_name = match colour {
        cube::Colour::Blue => "Blue Cube",
        cube::Colour::Red => "Red Cube",
    };
    (0..count)
        .map(|_| {
            let e = add_model_to_world(model_name, &models, &mut world, None)
                .expect("Unable to add Red Cube");
            add_cube_physics(&mut world, &mut physics_context, e);

            // Make un-renderable
            let mut entry = world.entry(e).unwrap();
            entry.add_component(Cube { colour });
            let mut mesh = entry.get_component_mut::<Mesh>().unwrap();
            mesh.should_render = false;
            e
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use hotham::{
        components::{Collider, Mesh, RigidBody},
        gltf_loader,
        resources::{
            render_context::create_descriptor_set_layouts, vulkan_context::VulkanContext,
            PhysicsContext,
        },
        schedule_functions::physics_step,
    };
    use legion::{IntoQuery, Resources, Schedule, World};
    use nalgebra::vector;

    use crate::components::Cube;

    use super::*;

    #[test]
    pub fn test_cube_spawner() {
        // SETUP
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();
        let glb_bufs: Vec<&[u8]> = vec![include_bytes!("../../assets/beat_saber.glb")];
        let models: Models =
            gltf_loader::load_models_from_glb(&glb_bufs, &vulkan_context, &set_layouts).unwrap();

        let red_cubes = create_cubes(
            10,
            cube::Colour::Red,
            &models,
            &mut world,
            &mut physics_context,
        );
        let blue_cubes = create_cubes(
            10,
            cube::Colour::Blue,
            &models,
            &mut world,
            &mut physics_context,
        );

        let mut resources = Resources::default();
        resources.insert(physics_context);
        let mut schedule = Schedule::builder()
            .add_system(cube_spawner_system(red_cubes, blue_cubes, 50))
            .add_thread_local_fn(physics_step)
            .build();

        schedule.execute(&mut world, &mut resources);

        // ASSERTIONS

        let mut query = <(&Mesh, &Collider, &RigidBody, &Cube)>::query();
        let mut renderable = query
            .iter(&world)
            .filter(|(m, c, r, cube)| m.should_render)
            .collect::<Vec<_>>();
        assert_eq!(renderable.len(), 1);

        {
            let (_mesh, _collider, rigid_body, cube) = renderable.pop().unwrap();
            // It should be red
            assert_eq!(cube.colour, cube::Colour::Red);

            let physics = resources.get::<PhysicsContext>().unwrap();
            let rigid_body = &physics.rigid_bodies[rigid_body.handle];
            let translation = rigid_body.translation();

            // It should be located -10 metres in the Z direction (eg. in the distance)
            // assert_eq!(translation.z, CUBE_STARTING_DISTANCE);
            // assert_eq!(translation.y, CUBE_STARTING_HEIGHT);

            // It should have a linear velocity of 1 in the Z direction (eg. towards the viewer)
            assert_eq!(rigid_body.linvel(), &CUBE_STARTING_VELOCITY);
        }

        for _ in 0..50 {
            schedule.execute(&mut world, &mut resources);
        }

        let mut query = <(&Mesh, &Collider, &RigidBody, &Cube)>::query();
        let mut blue_renderable = query
            .iter(&world)
            .filter(|(m, _c, _r, cube)| m.should_render && cube.colour == cube::Colour::Blue)
            .collect::<Vec<_>>();
        assert_eq!(blue_renderable.len(), 1);

        {
            let (_mesh, _collider, rigid_body, _cube) = blue_renderable.pop().unwrap();

            let physics = resources.get::<PhysicsContext>().unwrap();
            let rigid_body = &physics.rigid_bodies[rigid_body.handle];
            let translation = rigid_body.translation();

            // It should be located -10 metres in the Z direction (eg. in the distance)
            // assert_eq!(translation.z, CUBE_STARTING_DISTANCE);
            // assert_eq!(translation.y, CUBE_STARTING_HEIGHT);

            // It should have a linear velocity of 1 in the Z direction (eg. towards the viewer)
            assert_eq!(rigid_body.linvel(), &CUBE_STARTING_VELOCITY);
        }

        for _ in 0..201 {
            schedule.execute(&mut world, &mut resources);
        }

        let mut query = <(&Mesh, &Collider, &RigidBody, &Cube)>::query();
        let renderable = query
            .iter(&world)
            .filter(|(m, _c, _r, _cube)| m.should_render)
            .collect::<Vec<_>>();
        assert_eq!(renderable.len(), 4);
    }
}
