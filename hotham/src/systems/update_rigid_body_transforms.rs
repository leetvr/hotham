use legion::system;

use crate::{
    components::{RigidBody, Transform},
    resources::PhysicsContext,
};

#[system(for_each)]
pub fn update_rigid_body_transforms(
    rigid_body: &RigidBody,
    transform: &mut Transform,
    #[resource] physics_context: &PhysicsContext,
) {
    let rigid_body = &physics_context.rigid_bodies[rigid_body.handle];
    let position = rigid_body.position();

    // Update translation
    transform.translation.x = position.translation.x;
    transform.translation.y = position.translation.y;
    transform.translation.z = position.translation.z;

    // Update rotation
    let rotation: mint::Quaternion<f32> = (*position.rotation.quaternion()).into();
    transform.rotation = rotation.into();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schedule_functions::physics_step;
    use cgmath::{assert_relative_eq, vec3, Euler, Quaternion, Rad};
    use legion::{IntoQuery, Resources, Schedule, World};
    use nalgebra::{vector, Unit, UnitQuaternion};
    use rapier3d::{math::Isometry, prelude::RigidBodyBuilder};

    #[test]
    pub fn test_update_rigid_body_transforms_system() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let mut physics_context = PhysicsContext::default();

        let entity = world.push((Transform::default(),));
        let mut entry = world.entry(entity).unwrap();
        let mut rigid_body = RigidBodyBuilder::new_kinematic_position_based().build();
        let rotation: Quaternion<f32> = Euler {
            x: Rad(0.1),
            y: Rad(0.2),
            z: Rad(0.3),
        }
        .into();
        let rotation: mint::Quaternion<f32> = rotation.into();
        let rotation: nalgebra::Quaternion<f32> = rotation.into();
        let rotation: UnitQuaternion<f32> = Unit::new_normalize(rotation);
        let position = Isometry::from_parts(vector![1.0, 2.0, 3.0].into(), rotation);
        rigid_body.set_next_kinematic_position(position);

        let handle = physics_context.rigid_bodies.insert(rigid_body);

        entry.add_component(RigidBody { handle });

        resources.insert(physics_context);

        let mut schedule = Schedule::builder()
            .add_thread_local_fn(physics_step)
            .add_system(update_rigid_body_transforms_system())
            .build();

        schedule.execute(&mut world, &mut resources);
        schedule.execute(&mut world, &mut resources);
        schedule.execute(&mut world, &mut resources);
        schedule.execute(&mut world, &mut resources);

        let mut query = <&Transform>::query();
        let transform = query.get(&world, entity).unwrap();
        let rotation = Euler::from(transform.rotation);
        assert_relative_eq!(transform.translation, vec3(1.0, 2.0, 3.0));
        assert_relative_eq!(
            Euler {
                x: Rad(0.1),
                y: Rad(0.2),
                z: Rad(0.3)
            },
            rotation
        );
    }
}
