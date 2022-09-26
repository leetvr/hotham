use hecs::{PreparedQuery, World};
use nalgebra::UnitQuaternion;

use crate::{
    components::{LocalTransform, RigidBody},
    resources::PhysicsContext,
    Engine,
};

/// Walks through each pair of `RigidBody`s and `LocalTransform`s and sets the `LocalTransform` accordingly
pub fn update_local_transform_with_rigid_body_system(engine: &mut Engine) {
    let world = &mut engine.world;
    let physics_context = &mut engine.physics_context;
    update_local_transform_with_rigid_body_system_inner(world, physics_context);
}

pub(crate) fn update_local_transform_with_rigid_body_system_inner(
    world: &mut World,
    physics_context: &PhysicsContext,
) {
    for (_, (rigid_body, local_transform)) in world.query_mut::<(&RigidBody, &mut LocalTransform)>()
    {
        let rigid_body = &physics_context.rigid_bodies[rigid_body.handle];
        let position = rigid_body.position();

        // Update translation
        local_transform.translation.x = position.translation.x;
        local_transform.translation.y = position.translation.y;
        local_transform.translation.z = position.translation.z;

        // Update rotation
        local_transform.rotation = UnitQuaternion::new_normalize(*position.rotation.quaternion());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use hecs::World;
    use nalgebra::{vector, UnitQuaternion};
    use rapier3d::{
        math::Isometry,
        prelude::{RigidBodyBuilder, RigidBodyType},
    };

    #[test]
    pub fn test_update_local_transform_with_rigid_body_system() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        let entity = world.spawn((LocalTransform::default(),));
        let mut rigid_body = RigidBodyBuilder::new(RigidBodyType::KinematicPositionBased).build();
        let rotation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
        let position = Isometry::from_parts(vector![1.0, 2.0, 3.0].into(), rotation);
        rigid_body.set_next_kinematic_position(position);

        let handle = physics_context.rigid_bodies.insert(rigid_body);
        world.insert_one(entity, RigidBody { handle }).unwrap();

        let mut query = PreparedQuery::<(&RigidBody, &mut LocalTransform)>::default();

        // Run the schedule 4 times. Why 4 times? I can't remember.
        tick(&mut physics_context, &mut query, &mut world);
        tick(&mut physics_context, &mut query, &mut world);
        tick(&mut physics_context, &mut query, &mut world);
        tick(&mut physics_context, &mut query, &mut world);

        let local_transform = world.get::<LocalTransform>(entity).unwrap();
        assert_relative_eq!(local_transform.translation, vector![1.0, 2.0, 3.0]);
        assert_relative_eq!(local_transform.rotation, rotation);
    }

    fn tick(
        physics_context: &mut PhysicsContext,
        query: &mut PreparedQuery<(&RigidBody, &mut LocalTransform)>,
        world: &mut World,
    ) {
        physics_context.update();
        update_local_transform_with_rigid_body_system_inner(world, physics_context);
    }
}
