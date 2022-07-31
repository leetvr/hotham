use nalgebra::Matrix4;

use crate::components::{GlobalTransform, LocalTransform};
use hecs::{PreparedQuery, World};

/// Update transform matrix system
/// Walks through each LocalTransform and applies it to a 4x4 matrix used by the vertex shader
pub fn update_global_transform_system(
    query: &mut PreparedQuery<(&LocalTransform, &mut GlobalTransform)>,
    world: &mut World,
) {
    for (_, (transform, transform_matrix)) in query.query_mut(world) {
        transform_matrix.0 = Matrix4::new_translation(&transform.translation)
            * Matrix4::from(transform.rotation)
            * Matrix4::new_nonuniform_scaling(&transform.scale);
    }
}

#[cfg(test)]
mod tests {
    use crate::components::LocalTransform;
    use approx::assert_relative_eq;
    use nalgebra::{vector, UnitQuaternion};

    use super::*;

    #[test]
    pub fn test_update_global_transform_system() {
        let mut world = World::new();
        let transform = LocalTransform::default();
        let transform_matrix = GlobalTransform::default();

        let entity = world.spawn((transform, transform_matrix));

        {
            let matrix = world.get_mut::<GlobalTransform>(entity).unwrap();
            assert_eq!(matrix.0, Matrix4::identity());
        }

        let test_translation = vector![5.0, 1.0, 2.0];
        let test_rotation = UnitQuaternion::from_euler_angles(0.3, 0.3, 0.3);

        {
            let mut transform = world.get_mut::<LocalTransform>(entity).unwrap();
            transform.translation = test_translation;
            transform.rotation = test_rotation;
            transform.scale = test_translation;
        }

        update_global_transform_system(&mut Default::default(), &mut world);

        let expected_matrix = Matrix4::new_translation(&test_translation)
            * Matrix4::from(test_rotation)
            * Matrix4::new_nonuniform_scaling(&vector![5.0, 1.0, 2.0]);

        let matrix = world.get_mut::<GlobalTransform>(entity).unwrap();
        assert_relative_eq!(matrix.0, expected_matrix);
    }
}
