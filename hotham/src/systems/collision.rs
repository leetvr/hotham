use crate::{components::Collider, resources::PhysicsContext, util::u64_to_entity};
use legion::system;

#[system(for_each)]
pub fn collision(collider: &mut Collider, #[resource] physics_context: &mut PhysicsContext) {
    // Clear out any collisions from previous frames.
    collider.collisions_this_frame.clear();
    for (a, b, intersecting) in physics_context
        .narrow_phase
        .intersections_with(collider.handle)
    {
        if intersecting {
            let other = if a == collider.handle { b } else { a };
            let other_collider = &physics_context.colliders[other];
            let other_entity = u64_to_entity(other_collider.user_data as _);
            println!("{:?} is intersecting with {:?}!", collider.handle, other);
            collider.collisions_this_frame.push(other_entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Collider, Info};
    use crate::resources::PhysicsContext;
    use crate::util::entity_to_u64;

    use legion::{Entity, Resources, Schedule, World};
    use rapier3d::math::Isometry;
    use rapier3d::na as nalgebra;
    use rapier3d::prelude::*;
    use rapier3d::{na::vector, prelude::ColliderBuilder};

    #[test]
    pub fn test_collision() {
        let mut physics_context = PhysicsContext::default();
        let mut world = World::default();
        let mut resources = Resources::default();

        let a = make_collider(
            ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                .position(Isometry::new(
                    vector![0.5, 0.0, 0.0],
                    vector![0.0, 0.0, 0.0],
                ))
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .sensor(true)
                .build(),
            &mut world,
            0,
            &mut physics_context,
        );
        let b = make_collider(
            ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                .sensor(true)
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::CONTACT_EVENTS | ActiveEvents::INTERSECTION_EVENTS)
                .build(),
            &mut world,
            0,
            &mut physics_context,
        );

        physics_context.update();

        resources.insert(physics_context);

        // do something that would cause a and b to collide
        let mut schedule = Schedule::builder().add_system(collision_system()).build();
        schedule.execute(&mut world, &mut resources);

        let a_entry = world.entry(a).unwrap();
        let a_collider = a_entry.get_component::<Collider>().unwrap();
        assert!(a_collider.collisions_this_frame.contains(&b));
    }

    fn make_collider(
        mut collider: rapier3d::geometry::Collider,
        world: &mut World,
        node_id: usize,
        physics_context: &mut PhysicsContext,
    ) -> Entity {
        let entity = world.push((Info {
            name: format!("Node {}", node_id),
            node_id,
        },));
        let mut entry = world.entry(entity).unwrap();
        collider.user_data = entity_to_u64(entity) as _;
        let rigid_body = RigidBodyBuilder::new_dynamic().build();
        let rigid_body_handle = physics_context.rigid_bodies.insert(rigid_body);

        let a_collider_handle = physics_context.colliders.insert_with_parent(
            collider,
            rigid_body_handle,
            &mut physics_context.rigid_bodies,
        );
        entry.add_component(Collider {
            collisions_this_frame: vec![],
            handle: a_collider_handle,
        });

        entity
    }
}
