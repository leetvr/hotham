use crate::{components::Collider, resources::PhysicsContext};
use hecs::{PreparedQuery, World};

pub fn collision_system(
    query: &PreparedQuery<&mut Collider>,
    world: &mut World,
    physics_context: &mut PhysicsContext,
) {
    for (_, collider) in query.query_mut(world) {
        // Clear out any collisions from previous frames.
        collider.collisions_this_frame.clear();
        for (a, b, intersecting) in physics_context
            .narrow_phase
            .intersections_with(collider.handle)
        {
            if intersecting {
                let other = if a == collider.handle { b } else { a };
                let other_collider = &physics_context.colliders[other];
                let other_entity =
                    unsafe { world.find_entity_from_id(other_collider.user_data as _) };
                collider.collisions_this_frame.push(other_entity);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{Collider, Info};
    use crate::resources::PhysicsContext;

    use hecs::Entity;
    use rapier3d::prelude::*;
    use rapier3d::{
        math::Isometry,
        na::{self as nalgebra, vector},
    };

    #[test]
    pub fn test_collision() {
        let mut physics_context = PhysicsContext::default();
        let mut world = World::default();

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

        // do something that would cause a and b to collide
        let query = PreparedQuery::<&mut Collider>::default();
        collision_system(&query, &mut world, &mut physics_context);

        let a_collider = world.get_mut::<&Collider>(a).unwrap();
        assert!(a_collider.collisions_this_frame.contains(&b));
    }

    fn make_collider(
        mut collider: rapier3d::geometry::Collider,
        world: &mut World,
        node_id: usize,
        physics_context: &mut PhysicsContext,
    ) -> Entity {
        let entity = world.spawn((Info {
            name: format!("Node {}", node_id),
            node_id,
        },));
        collider.user_data = entity.id() as _;
        let rigid_body = RigidBodyBuilder::new_dynamic().build();
        let components = physics_context.get_rigid_body_and_collider(entity, rigid_body, collider);
        world.insert(entity, components);

        entity
    }
}
