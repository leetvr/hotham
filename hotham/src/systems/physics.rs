use physics_context::PhysicsContext;

use crate::{
    components::{Collider, GlobalTransform, LocalTransform, Parent, PhysicsControlled, RigidBody},
    contexts::physics_context,
    Engine,
};

/// Update the physics simulation and synchronise it with the game simulation.
///
/// There are two ways we synchronize between the physics simulation and the game:
///
/// 1. **Game controlled** - physics objects have their positions set by the **game** simulation
/// 1. **Physics controlled** - game objects have their positions set by the **physics** simulation
///
/// You can indicate to [`physics_system`] how you'd like this entity to be treated by adding or removing
/// the [`PhysicsControlled`] component to an entity. If an entity has the [`PhysicsControlled`] component, you
/// are indicating that you want this entity's position in the game simulation to be entirely controlled
/// by the physics simulation.
///
/// There are a couple of situations where this may not make sense (eg. if a rigid body is kinematic position based)
/// so it is up to you to pick the right body type on your [`rapier3d::dynamics::RigidBody`] or you'll have a Very Bad Time.
///
/// Physics controlled objects will have their [`crate::components::LocalTransform`] updated directly. What this means
/// is that the entity should *NOT* have a `Parent*, or else its position in the game simulation will not be updated
/// and you will have a Very Bad Time.
pub fn physics_system(engine: &mut Engine) {
    physics_system_inner(&mut engine.physics_context, &mut engine.world);
}

pub(crate) fn physics_system_inner(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    // First, update any game controlled rigid bodies.
    update_game_controlled_rigid_bodies(physics_context, world);

    // Then update any game controlled colliders that *do not* have a rigid body.
    update_game_controlled_colliders(physics_context, world);

    // Next, update the physics simulation.
    physics_context.update();

    // Now update any physics controlled rigid bodies.
    update_physics_controlled_rigid_bodies(physics_context, world);

    // Lastly check for collisions and update the relevant colliders.
    update_collisions(world, physics_context);
}

fn update_game_controlled_rigid_bodies(
    physics_context: &mut PhysicsContext,
    world: &mut hecs::World,
) {
    for (_, (rigid_body, global_transform)) in
        world.query_mut::<hecs::Without<(&RigidBody, &GlobalTransform), &PhysicsControlled>>()
    {
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body.handle)
            .unwrap();
        rigid_body.set_next_kinematic_position(global_transform.to_isometry());
    }
}

fn update_game_controlled_colliders(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    for (_, (collider, global_transform)) in world.query_mut::<hecs::Without<
        (&mut Collider, &GlobalTransform),
        (&PhysicsControlled, &RigidBody)>
    >() {
        let collider = physics_context.colliders.get_mut(collider.handle).unwrap();
        collider.set_position(global_transform.to_isometry());
    }
}

fn update_physics_controlled_rigid_bodies(
    physics_context: &PhysicsContext,
    world: &mut hecs::World,
) {
    for (_, (rigid_body, local_transform)) in
        world.query_mut::<hecs::With<
            hecs::Without<(&RigidBody, &mut LocalTransform), &Parent>,
            &PhysicsControlled,
        >>()
    {
        let position_in_physics_simulation = physics_context
            .rigid_bodies
            .get(rigid_body.handle)
            .unwrap()
            .position();
        local_transform.update_from_isometry(position_in_physics_simulation);
    }
}

// TODO: This is *very* slow! Rapier has much better ways of doing this.
fn update_collisions(world: &hecs::World, physics_context: &mut PhysicsContext) {
    for (_, collider) in world.query::<&mut Collider>().iter() {
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
    use approx::assert_relative_eq;
    use glam::{Affine3A, Quat, Vec3};
    use hecs::World;
    use rapier3d::prelude::{
        ActiveCollisionTypes, ActiveEvents, ColliderBuilder, RigidBodyBuilder,
    };

    use crate::{
        components::{
            Collider, GlobalTransform, Info, LocalTransform, PhysicsControlled, RigidBody,
        },
        contexts::PhysicsContext,
    };

    use super::physics_system_inner;

    #[test]
    /// Test that game controlled rigid bodies (eg. hands) have their positions in the physics simulation set by their game position.
    pub fn test_game_controlled_rigid_body() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();

        let expected_transform = GlobalTransform(Affine3A::from_translation([1., 2., 3.].into()));

        // Create our test entity.
        let rigid_body_entity = world.spawn((
            RigidBody::new(
                physics_context.rigid_bodies.insert(
                    RigidBodyBuilder::kinematic_position_based()
                        .translation([1.98, 11., 59.].into())
                        .build(),
                ),
            ),
            expected_transform,
        ));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Get the position
        let rigid_body = physics_context
            .get_rigid_body(&world, rigid_body_entity)
            .unwrap();
        assert_relative_eq!(rigid_body.position(), &expected_transform.to_isometry());
    }

    #[test]
    /// Test that game controlled colliders without rigid bodies (eg. walls, sensors) need their positions set by their global transform.
    pub fn test_game_controlled_collider() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();

        let expected_transform = GlobalTransform(Affine3A::from_translation([1., 2., 3.].into()));

        // Create our test entity.
        let collider_entity = world.spawn((
            Collider::new(
                physics_context.colliders.insert(
                    ColliderBuilder::ball(1.0)
                        .translation([0.1512, 22., 44.].into())
                        .build(),
                ),
            ),
            expected_transform,
        ));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Get the position
        let collider = physics_context
            .get_collider(&world, collider_entity)
            .unwrap();
        assert_relative_eq!(collider.position(), &expected_transform.to_isometry());
    }

    #[test]
    /// Test that physics controlled rigid bodies have their positions set by the physics simulation.
    pub fn test_physics_controlled_rigid_body() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();

        let expected_position = LocalTransform::from_rotation_translation(
            Quat::from_axis_angle(Vec3::X, std::f32::consts::PI),
            [1., 2., 3.].into(),
        );

        // Create our test entity.
        let rigid_body_entity = world.spawn((
            RigidBody::new(
                physics_context.rigid_bodies.insert(
                    RigidBodyBuilder::dynamic()
                        .position(expected_position.to_isometry())
                        .build(),
                ),
            ),
            LocalTransform::default(),
            PhysicsControlled {},
        ));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Get the local transform
        let local_transform = world.get::<&LocalTransform>(rigid_body_entity).unwrap();
        assert_eq!(*local_transform, expected_position);
    }

    #[test]
    pub fn test_collision() {
        let mut physics_context = PhysicsContext::default();
        let mut world = hecs::World::default();
        let position =
            LocalTransform::from_rotation_translation(Quat::IDENTITY, [0.5, 0., 0.].into());

        let a = make_collider(
            ColliderBuilder::cuboid(1.0, 1.0, 1.0)
                .position(position.to_isometry())
                .active_collision_types(ActiveCollisionTypes::all())
                .active_events(ActiveEvents::COLLISION_EVENTS)
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
                .active_events(ActiveEvents::COLLISION_EVENTS)
                .build(),
            &mut world,
            0,
            &mut physics_context,
        );

        // do something that would cause a and b to collide
        physics_system_inner(&mut physics_context, &mut world);

        let a_collider = world.get::<&mut Collider>(a).unwrap();
        assert!(a_collider.collisions_this_frame.contains(&b));
    }

    fn make_collider(
        mut collider: rapier3d::geometry::Collider,
        world: &mut World,
        node_id: usize,
        physics_context: &mut PhysicsContext,
    ) -> hecs::Entity {
        let entity = world.spawn((Info {
            name: format!("Node {}", node_id),
            node_id,
        },));
        collider.user_data = entity.id() as _;
        let rigid_body = RigidBodyBuilder::dynamic().build();
        let components =
            physics_context.create_rigid_body_and_collider(entity, rigid_body, collider);
        world.insert(entity, components).unwrap();

        entity
    }
}
