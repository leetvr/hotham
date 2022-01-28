use hecs::{PreparedQuery, World};
use nalgebra::UnitQuaternion;

use crate::{
    components::{RigidBody, Transform},
    resources::PhysicsContext,
};

pub fn update_rigid_body_transforms_system(
    query: &mut PreparedQuery<(&RigidBody, &mut Transform)>,
    world: &mut World,
    physics_context: &PhysicsContext,
) {
    for (_, (rigid_body, transform)) in query.query_mut(world) {
        let rigid_body = &physics_context.rigid_bodies[rigid_body.handle];
        let position = rigid_body.position();

        // Update translation
        transform.translation.x = position.translation.x;
        transform.translation.y = position.translation.y;
        transform.translation.z = position.translation.z;

        // Update rotation
        transform.rotation = UnitQuaternion::new_normalize(*position.rotation.quaternion());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;
    use hecs::World;
    use nalgebra::{vector, UnitQuaternion};
    use rapier3d::{math::Isometry, prelude::RigidBodyBuilder};

    #[test]
    pub fn test_update_rigid_body_transforms_system() {
        let mut world = World::default();
        let mut physics_context = PhysicsContext::default();

        let entity = world.spawn((Transform::default(),));
        let mut rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
        let rotation = UnitQuaternion::from_euler_angles(0.1, 0.2, 0.3);
        let position = Isometry::from_parts(vector![1.0, 2.0, 3.0].into(), rotation.clone());
        rigid_body.set_next_kinematic_position(position);

        let handle = physics_context.rigid_bodies.insert(rigid_body);
        world.insert_one(entity, RigidBody { handle });

        let mut query = PreparedQuery::<(&RigidBody, &mut Transform)>::default();

        // Run the schedule 4 times. Why 4 times? I can't remember.
        schedule(&mut physics_context, &mut query, &mut world);
        schedule(&mut physics_context, &mut query, &mut world);
        schedule(&mut physics_context, &mut query, &mut world);
        schedule(&mut physics_context, &mut query, &mut world);

        let transform = world.get::<&Transform>(entity).unwrap();
        assert_relative_eq!(transform.translation, vector![1.0, 2.0, 3.0]);
        assert_relative_eq!(transform.rotation, rotation);
    }

    fn schedule(
        physics_context: &mut PhysicsContext,
        query: &mut PreparedQuery<(&RigidBody, &mut Transform)>,
        world: &mut World,
    ) {
        physics_context.update();
        update_rigid_body_transforms_system(query, world, physics_context);
    }
}
