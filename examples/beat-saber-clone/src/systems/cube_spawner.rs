use hotham::{
    components::{Mesh, RigidBody, Transform, Visible},
    gltf_loader::add_model_to_world,
    resources::{
        physics_context,
        render_context::DescriptorSetLayouts,
        vulkan_context::{self, VulkanContext},
        PhysicsContext,
    },
};
use legion::{
    component, system,
    systems::CommandBuffer,
    world::{EntryMut, SubWorld},
    Entity, EntityStore, IntoQuery, World,
};
use nalgebra::{vector, Vector3};
use rand::Rng;
use rapier3d::prelude::{ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder};

use crate::{
    components::{Colour, Cube},
    Models,
};

const CUBE_STARTING_VELOCITY: Vector3<f32> = Vector3::new(0., 0., 3.00);
const CUBE_STARTING_DISTANCE: f32 = -10.;
const CUBE_STARTING_HEIGHT: f32 = 1.2;

#[system]
#[read_component(Entity)]
#[read_component(Colour)]
#[write_component(RigidBody)]
#[write_component(Visible)]
pub fn cube_spawner(
    world: &mut SubWorld,
    command_buffer: &mut CommandBuffer,
    #[state] probability: &usize,
    #[resource] physics_context: &mut PhysicsContext,
) {
    let mut rng = rand::thread_rng();
    let r = rng.gen_range(0..*probability);

    if r == 0 {
        activate_cube(Colour::Red, world, physics_context, command_buffer);
    }

    if r == 1 {
        activate_cube(Colour::Blue, world, physics_context, command_buffer);
    }
}

fn activate_cube(
    colour: Colour,
    world: &mut SubWorld,
    physics_context: &mut PhysicsContext,
    command_buffer: &mut CommandBuffer,
) {
    let mut query = <(Entity, &Colour, &RigidBody)>::query()
        .filter(component::<Cube>() & !component::<Visible>());
    let query = query.iter(world);
    let (entity, _, rigid_body) = query.filter(|(_, c, _)| c == &&colour).next().unwrap();
    let entity = entity.clone();

    let mut rng = rand::thread_rng();
    println!("Activating {:?} cube: {:?}", colour, rigid_body.handle);
    let rigid_body = &mut physics_context.rigid_bodies[rigid_body.handle];
    let x_offset = rng.gen_range(-1.0..1.0);

    rigid_body.set_linvel(CUBE_STARTING_VELOCITY, true);
    rigid_body.set_translation(
        vector!(x_offset, CUBE_STARTING_HEIGHT, CUBE_STARTING_DISTANCE),
        true,
    );

    // Give the cube a Visible component
    command_buffer.add_component(entity, Visible {})
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
    models: &Models,
    mut world: &mut World,
    mut physics_context: &mut PhysicsContext,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &DescriptorSetLayouts,
) -> Vec<Entity> {
    let mut entities = Vec::new();
    for (colour, model_name) in [(Colour::Blue, "Blue Cube"), (Colour::Red, "Red Cube")] {
        for _ in 0..count {
            let entity = add_model_to_world(
                model_name,
                &models,
                &mut world,
                None,
                &vulkan_context,
                &descriptor_set_layouts,
            )
            .expect("Unable to add Red Cube");
            add_cube_physics(&mut world, &mut physics_context, entity);

            // Make un-renderable
            let mut entry = world.entry(entity).unwrap();
            entry.add_component(Cube {});
            entry.add_component(colour);
            entry.remove_component::<Visible>();

            drop(entry);
            entities.push(entity);
        }
    }

    entities
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
    };
    use legion::{IntoQuery, Resources, Schedule, World};

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

        create_cubes(
            10,
            &models,
            &mut world,
            &mut physics_context,
            &vulkan_context,
            &set_layouts,
        );

        let mut resources = Resources::default();
        resources.insert(physics_context);
        let mut schedule = Schedule::builder()
            .add_system(cube_spawner_system(2))
            .build();

        let mut red_found = 0;
        let mut blue_found = 0;
        let mut searches = 0;

        while searches < 10 && (red_found == 0 || blue_found == 0) {
            let mut query = <(&RigidBody, &Colour)>::query().filter(
                component::<Mesh>()
                    & component::<Collider>()
                    & component::<Cube>()
                    & component::<Visible>(),
            );
            schedule.execute(&mut world, &mut resources);

            // ASSERTIONS
            let mut renderable = query.iter(&world).collect::<Vec<_>>();
            let (rigid_body, colour) = renderable.pop().unwrap();

            let physics = resources.get::<PhysicsContext>().unwrap();
            let rigid_body = &physics.rigid_bodies[rigid_body.handle];
            let translation = rigid_body.translation();

            // It should be located -10 metres in the Z direction (eg. in the distance)
            assert_eq!(translation.z, CUBE_STARTING_DISTANCE);
            assert_eq!(translation.y, CUBE_STARTING_HEIGHT);

            // It should have a linear velocity of 1 in the Z direction (eg. towards the viewer)
            assert_eq!(rigid_body.linvel(), &CUBE_STARTING_VELOCITY);

            match colour {
                Colour::Red => red_found += 1,
                Colour::Blue => blue_found += 1,
            };
            searches += 1;
        }

        assert!(red_found >= 1);
        assert!(blue_found >= 1);
    }
}
