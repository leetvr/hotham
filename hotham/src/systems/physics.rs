use physics_context::PhysicsContext;
use rapier3d::prelude::{ActiveEvents, ColliderBuilder, InteractionGroups, RigidBodyBuilder};

use crate::{
    components::{
        physics::Impulse,
        physics::{AdditionalMass, BodyType, RigidBody, Teleport},
        Collider, GlobalTransform, LocalTransform, Parent,
    },
    contexts::physics_context,
    util::{glam_vec_from_na, na_vector_from_glam},
    Engine,
};

/// A private wrapper around a rapier rigid-body handle. This allows us to keep the implementation details of Rapier private
/// and also easily find rigid-bodies that have not yet been created in Rapier.
struct RigidBodyHandle(rapier3d::prelude::RigidBodyHandle);

/// A private wrapper around a rapier collider handle. This allows us to keep the implementation details of Rapier private
/// and also easily find colliders that have not yet been created in Rapier.
struct ColliderHandle(rapier3d::prelude::ColliderHandle);

/// Update the physics simulation and synchronise it with the game simulation.
///
/// There are two ways we synchronize between the physics simulation and the game:
///
/// 1. **Game controlled** - physics objects have their positions set by the **game** simulation
/// 1. **Physics controlled** - game objects have their positions set by the **physics** simulation
///
/// You can indicate to [`physics_system`] how you'd like this entity to be treated by changing the `body_type` field
/// on a [`RigidBody`]. Setting the `body_type` to [`BodyType::Dynamic`] will result in the entity having its [`GlobalTransform`]
/// overwritten by its position in the physics simulation - any updates to [`LocalTransform`] or [`GlobalTransform`] will be overwritten.
///
/// Any other kind of body is treated as *game controlled* - that is, updating the entity's [`LocalTransform`] will not be overwritten
/// and the position of the entity in the physics simulation will be updated based on its [`GlobalTransform`] (all transforms in the
/// physics simulation are in global space).
///
/// ## Panics
/// Trying to create a [`RigidBody`] with a `body_type` of [`BodyType::Dynamic`], or change an existing [`RigidBody`]'s `body_type` to
/// be [`BodyType::Dynamic`] on a [`hecs::Entity`] that has a [`Parent`] component will cause a panic.
///
/// This is not allowed as it would cause a conflict in attempting to determine the entity's final [`GlobalTransform`] due to the way
/// [`Parent`]s are handled in [`super::update_global_transform_with_parent_system`].
pub fn physics_system(engine: &mut Engine) {
    physics_system_inner(&mut engine.physics_context, &mut engine.world);
}

pub(crate) fn physics_system_inner(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    // First, see if there are any rigid-bodies or colliders in the world that don't currently have a handle in rapier.
    create_handles(physics_context, world);

    // Next, update any game controlled rigid bodies.
    update_physics_from_world(physics_context, world);

    // Next, update the physics simulation.
    physics_context.update();

    // Now update any physics controlled rigid bodies.
    update_world_from_physics(physics_context, world);
}

fn create_handles(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    create_rigid_bodies(world, physics_context);
    create_colliders(world, physics_context);
}

fn create_rigid_bodies(world: &mut hecs::World, physics_context: &mut PhysicsContext) {
    let mut command_buffer = hecs::CommandBuffer::new();

    for (entity, (r, parent, global_transform)) in world
        .query::<(&RigidBody, Option<&Parent>, &GlobalTransform)>()
        .without::<&RigidBodyHandle>()
        .iter()
    {
        if r.body_type == BodyType::Dynamic && parent.is_some() {
            panic!(
                "[HOTHAM-PHYSICS] ERROR - Entities with parents cannot have dynamic rigid bodies: {:?}",
                entity
            );
        }

        let mut rigid_body = RigidBodyBuilder::new(r.body_type.into())
            .additional_mass(r.mass)
            .position(global_transform.to_isometry())
            .linvel(na_vector_from_glam(r.linear_velocity))
            .user_data(entity.to_bits().get() as _)
            .build();
        rigid_body.recompute_mass_properties_from_colliders(&physics_context.colliders);
        let handle = RigidBodyHandle(physics_context.rigid_bodies.insert(rigid_body));
        command_buffer.insert_one(entity, handle);
    }

    command_buffer.run_on(world);
}

fn create_colliders(world: &mut hecs::World, physics_context: &mut PhysicsContext) {
    let mut command_buffer = hecs::CommandBuffer::new();

    for (entity, (c, rigid_body_handle)) in world
        .query::<(&mut Collider, Option<&RigidBodyHandle>)>()
        .without::<&ColliderHandle>()
        .iter()
    {
        let mut collider = ColliderBuilder::new(c.shape.clone())
            .user_data(entity.to_bits().get() as _)
            .sensor(c.sensor)
            .active_collision_types(c.active_collision_types)
            .active_events(ActiveEvents::all())
            .collision_groups(InteractionGroups::new(
                c.collision_groups,
                c.collision_filter,
            ))
            .build();

        // This is a bit odd, but a developer can override the mass of a collider if they choose.
        // To avoid having the mass overridden every frame, we check to see if the developer provided a mass on creation
        // If not, then we set the mass on the collider *component* to be whatever the computed mass of the shape is. That way
        // the collider's mass will be happily set to the *correct* value each frame.
        if c.mass != 0. {
            collider.set_mass(c.mass);
        } else {
            c.mass = collider.mass();
        }

        let handle = if let Some(handle) = rigid_body_handle {
            collider.set_translation_wrt_parent(na_vector_from_glam(c.offset_from_parent));
            physics_context.colliders.insert_with_parent(
                collider,
                handle.0,
                &mut physics_context.rigid_bodies,
            )
        } else {
            physics_context.colliders.insert(collider)
        };

        command_buffer.insert_one(entity, ColliderHandle(handle));
    }

    command_buffer.run_on(world);
}

fn update_physics_from_world(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    update_rigid_bodies_from_world(physics_context, world);
    update_colliders_from_world(physics_context, world);
}

fn update_rigid_bodies_from_world(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    let mut command_buffer = hecs::CommandBuffer::new();
    for (entity, (rigid_body_component, rigid_body_handle, global_transform, parent)) in world
        .query::<(
            &RigidBody,
            &RigidBodyHandle,
            &GlobalTransform,
            Option<&Parent>,
        )>()
        .iter()
    {
        let body_type = rigid_body_component.body_type;
        let rigid_body = physics_context
            .rigid_bodies
            .get_mut(rigid_body_handle.0)
            .unwrap();

        if body_type == BodyType::Dynamic && parent.is_some() {
            panic!(
                "[HOTHAM-PHYSICS] ERROR - Entities with parents cannot have dynamic rigid bodies: {:?}",
                entity
            );
        }

        rigid_body.set_body_type(body_type.into());

        // We cheat here by just comparing with the first member of the array, as we don't yet support locking
        // individual rotations.
        if rigid_body_component.lock_rotations != rigid_body.is_rotation_locked()[0] {
            rigid_body.lock_rotations(rigid_body_component.lock_rotations, true)
        }

        let component_linear_velocity = na_vector_from_glam(rigid_body_component.linear_velocity);

        match body_type {
            BodyType::KinematicPositionBased => {
                rigid_body.set_next_kinematic_position(global_transform.to_isometry())
            }
            BodyType::KinematicVelocityBased => {
                if rigid_body.linvel() != &component_linear_velocity {
                    rigid_body.set_linvel(component_linear_velocity, true);
                }

                // Teleport the entity
                if world.get::<&Teleport>(entity).is_ok() {
                    command_buffer.remove_one::<Teleport>(entity);
                    let next_position = global_transform.to_isometry();
                    println!("[HOTHAM-PHYSICS] Teleporting entity to {:?}", next_position);
                    rigid_body.set_position(next_position, true);
                }
            }
            BodyType::Dynamic => {
                // Update the linear velocity if it's been updated
                if rigid_body.linvel() != &component_linear_velocity {
                    rigid_body.set_linvel(component_linear_velocity, true);
                }

                // Teleport the entity
                if world.get::<&Teleport>(entity).is_ok() {
                    command_buffer.remove_one::<Teleport>(entity);
                    let next_position = global_transform.to_isometry();
                    println!("[HOTHAM-PHYSICS] Teleporting entity to {:?}", next_position);
                    rigid_body.set_position(next_position, true);
                }

                // Apply one-shot components
                if let Ok(additional_mass) = world.get::<&AdditionalMass>(entity).map(|a| a.value) {
                    command_buffer.remove_one::<AdditionalMass>(entity);
                    println!(
                        "[HOTHAM-PHYSICS] Applying additional mass of {:?}",
                        additional_mass
                    );
                    rigid_body.set_additional_mass(additional_mass, true);
                    rigid_body.recompute_mass_properties_from_colliders(&physics_context.colliders);
                }

                if let Ok(impulse) = world.get::<&Impulse>(entity).map(|i| i.value) {
                    command_buffer.remove_one::<Impulse>(entity);
                    println!("[HOTHAM-PHYSICS] Applying impulse of {:?}", impulse);
                    rigid_body.apply_impulse(na_vector_from_glam(impulse), true);
                }
            }
            _ => {}
        }
    }
    command_buffer.run_on(world);
}

fn update_colliders_from_world(physics_context: &mut PhysicsContext, world: &mut hecs::World) {
    for (_, (collider_component, collider_handle, global_transform, rigid_body)) in world
        .query_mut::<(
            &Collider,
            &ColliderHandle,
            &GlobalTransform,
            Option<&RigidBody>,
        )>()
    {
        let collider = &mut physics_context.colliders[collider_handle.0];

        // Only update position of colliders that don't have a rigid-body attached
        if rigid_body.is_none() {
            collider.set_position(global_transform.to_isometry());
        }

        let Collider {
            sensor,
            shape,
            collision_groups,
            collision_filter,
            active_collision_types,
            mass,
            offset_from_parent,
            restitution,
            collisions_this_frame: _, // we intentionally ignore this value to force us to handle all other properties
        } = collider_component;

        // Update the collider's other properties.
        collider.set_sensor(*sensor);
        collider.set_shape(shape.clone());
        collider.set_collision_groups(InteractionGroups::new(*collision_groups, *collision_filter));
        collider.set_mass(*mass);
        collider.set_restitution(*restitution);
        collider.set_active_collision_types(*active_collision_types);
        collider.set_translation_wrt_parent(na_vector_from_glam(*offset_from_parent));
    }
}

fn update_world_from_physics(physics_context: &PhysicsContext, world: &mut hecs::World) {
    for (_, (rigid_body_handle, rigid_body_component, local_transform)) in
        world.query_mut::<(&RigidBodyHandle, &mut RigidBody, &mut LocalTransform)>()
    {
        let rigid_body = &physics_context.rigid_bodies[rigid_body_handle.0];

        if rigid_body_component.body_type == BodyType::Dynamic
            || rigid_body_component.body_type == BodyType::KinematicVelocityBased
        {
            local_transform.update_from_isometry(rigid_body.position());
        }

        // Update the component's linear velocity.
        rigid_body_component.linear_velocity = glam_vec_from_na(rigid_body.linvel());

        // Update the component's mass
        rigid_body_component.mass = rigid_body.mass();
    }

    // Lastly check for collisions and update the relevant colliders.
    update_collisions(physics_context, world);
}

// TODO: This is *very* slow! Rapier has much better ways of doing this.
fn update_collisions(physics_context: &PhysicsContext, world: &hecs::World) {
    for (_, (collider, collider_handle)) in world.query::<(&mut Collider, &ColliderHandle)>().iter()
    {
        // Clear out any collisions from previous frames.
        collider.collisions_this_frame.clear();
        for (a, b, intersecting) in physics_context
            .narrow_phase
            .intersections_with(collider_handle.0)
        {
            if intersecting {
                let other = if a == collider_handle.0 { b } else { a };
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
    use approx::{assert_relative_eq, assert_relative_ne};
    use glam::{Affine3A, Quat, Vec3};
    use rapier3d::prelude::ActiveCollisionTypes;

    use crate::{
        components::{
            physics::Impulse,
            physics::{AdditionalMass, BodyType, RigidBody, Teleport},
            Collider, GlobalTransform, LocalTransform,
        },
        contexts::PhysicsContext,
        systems::physics::{ColliderHandle, RigidBodyHandle},
    };

    use super::physics_system_inner;

    #[test]
    /// Test that kinematic rigid bodies have their positions in the physics simulation set by their game position.
    pub fn test_kinematic_rigid_body() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();

        let expected_transform = GlobalTransform(Affine3A::from_translation([1., 2., 3.].into()));

        // Create our test entity.
        let rigid_body_entity = world.spawn((
            RigidBody {
                body_type: BodyType::KinematicPositionBased,
                ..Default::default()
            },
            expected_transform,
        ));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Get the position
        let handle = world.get::<&RigidBodyHandle>(rigid_body_entity).unwrap();
        let rigid_body = &physics_context.rigid_bodies[handle.0];
        assert_relative_eq!(rigid_body.position(), &expected_transform.to_isometry());
    }

    #[test]
    /// Test that game controlled colliders without rigid bodies (eg. walls, sensors) have their positions set by their global transform.
    pub fn test_game_controlled_collider() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();

        let expected_transform = GlobalTransform(Affine3A::from_translation([1., 2., 3.].into()));

        // Create our test entity.
        let collider_entity = world.spawn((Collider::default(), expected_transform));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Get the position
        let collider =
            &physics_context.colliders[world.get::<&ColliderHandle>(collider_entity).unwrap().0];
        assert_relative_eq!(collider.position(), &expected_transform.to_isometry());
    }

    #[test]
    /// Test that dynamic rigid bodies have their positions set by the physics simulation.
    pub fn test_dynamic_rigid_body() {
        let mut world = hecs::World::default();
        let mut physics_context = PhysicsContext::default();
        let expected_transform =
            LocalTransform::from_rotation_translation(Quat::default(), [1., 2., 3.].into());

        // Create our test entity.
        let rigid_body_entity = world.spawn((
            RigidBody {
                body_type: BodyType::Dynamic, // though this is part of default, we're just making this explicit
                ..Default::default()
            },
            expected_transform,
            GlobalTransform::from(expected_transform),
        ));

        // Run the system
        physics_system_inner(&mut physics_context, &mut world);

        // Check that the *INITIAL* position of the entity is what was originally inserted:
        {
            let handle = world.get::<&RigidBodyHandle>(rigid_body_entity).unwrap();
            let rigid_body = &physics_context.rigid_bodies[handle.0];
            assert_relative_eq!(rigid_body.position(), &expected_transform.to_isometry());

            let mut local_transform = world.get::<&mut LocalTransform>(rigid_body_entity).unwrap();

            // Make sure the local transform has not been changed yet:
            assert_relative_eq!(local_transform.to_affine(), expected_transform.to_affine());

            // Now the local transform back to default - this change will be ignored.
            *local_transform = LocalTransform::default();
        }

        // Run the system again
        physics_system_inner(&mut physics_context, &mut world);

        // Get the local transform
        {
            let local_transform = world.get::<&LocalTransform>(rigid_body_entity).unwrap();
            // Make sure it has *NOT* been changed.
            assert_relative_eq!(local_transform.to_affine(), expected_transform.to_affine());

            // Now add some velocity to the rigid body
            let mut rigid_body = world.get::<&mut RigidBody>(rigid_body_entity).unwrap();
            rigid_body.linear_velocity = Vec3::X * 1000.;
        }

        // Run the system again
        physics_system_inner(&mut physics_context, &mut world);

        // The body should now have moved, and the linear velocity should be unchanged as there are no forces affecting it.
        {
            let local_transform = world.get::<&LocalTransform>(rigid_body_entity).unwrap();
            // Make sure it has actually moved
            assert_relative_ne!(local_transform.to_affine(), expected_transform.to_affine());
            assert_ne!(*local_transform, LocalTransform::default());

            // Make sure the velocity has not changed
            let rigid_body = world.get::<&RigidBody>(rigid_body_entity).unwrap();
            assert_relative_eq!(rigid_body.linear_velocity, Vec3::X * 1000.);
        }

        // Now change it to a kinematic position-based rigid body and update its transform
        let expected_translation = [1., 2., 3.].into();
        {
            let mut local_transform = world.get::<&mut LocalTransform>(rigid_body_entity).unwrap();
            local_transform.translation = expected_translation;

            let mut rigid_body = world.get::<&mut RigidBody>(rigid_body_entity).unwrap();
            rigid_body.body_type = BodyType::KinematicPositionBased;
        }

        // Run the system again
        physics_system_inner(&mut physics_context, &mut world);

        let local_transform = world.get::<&LocalTransform>(rigid_body_entity).unwrap();
        assert_relative_eq!(local_transform.translation, expected_translation);
    }

    /// Test adding "one shot" components to add a specific behaviour to an entity, once
    #[test]
    pub fn test_one_shot_components() {
        let mut physics_context = PhysicsContext::default();
        let mut world = hecs::World::default();
        let entity = world.spawn((
            RigidBody {
                mass: 10.,
                ..Default::default()
            },
            AdditionalMass::new(100.),
            Impulse::new(Vec3::X * 1000.),
            GlobalTransform::default(),
            LocalTransform::default(),
        ));

        physics_system_inner(&mut physics_context, &mut world);

        // Make sure the one-shots applied.
        {
            let entity = world.entity(entity).unwrap();
            assert!(!entity.has::<Impulse>());
            assert!(!entity.has::<AdditionalMass>());

            let local_transform = entity.get::<&LocalTransform>().unwrap();
            assert!(local_transform.translation != Vec3::ZERO);

            let rigid_body = entity.get::<&RigidBody>().unwrap();
            assert_eq!(rigid_body.mass, 100.);
            assert!(rigid_body.linear_velocity != Vec3::ZERO);
        }

        // Now teleport it
        {
            world.insert_one(entity, Teleport {}).unwrap();
            let mut local_transform = world.get::<&mut LocalTransform>(entity).unwrap();
            local_transform.translation = [1., 2., 3.].into();
            let mut global_transform = world.get::<&mut GlobalTransform>(entity).unwrap();
            *global_transform = (*local_transform).into();

            // Slow the entity down so it doesn't just move away
            let mut rigid_body = world.get::<&mut RigidBody>(entity).unwrap();
            rigid_body.linear_velocity = Vec3::ZERO;
        }

        physics_system_inner(&mut physics_context, &mut world);

        // Make sure the one-shot applied.
        {
            let entity = world.entity(entity).unwrap();
            assert!(!entity.has::<Teleport>());
            let local_transform = entity.get::<&LocalTransform>().unwrap();
            assert_relative_eq!(local_transform.translation, [1., 2., 3.].into());
        }
    }

    #[test]
    pub fn test_collision() {
        let mut physics_context = PhysicsContext::default();
        let mut world = hecs::World::default();
        let local_transform =
            LocalTransform::from_rotation_translation(Quat::IDENTITY, [0.5, 0., 0.].into());

        let a = world.spawn((
            Collider {
                sensor: true,
                active_collision_types: ActiveCollisionTypes::FIXED_FIXED,
                ..Default::default()
            },
            local_transform,
            GlobalTransform::from(local_transform),
        ));
        let b = world.spawn((
            Collider {
                sensor: true,
                active_collision_types: ActiveCollisionTypes::FIXED_FIXED,
                ..Default::default()
            },
            local_transform,
            GlobalTransform::from(local_transform),
        ));

        // // do something that would cause a and b to collide
        physics_system_inner(&mut physics_context, &mut world);

        let a_collider = world.get::<&mut Collider>(a).unwrap();
        assert!(a_collider.collisions_this_frame.contains(&b));
    }
}
