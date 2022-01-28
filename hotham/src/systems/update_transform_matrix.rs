use nalgebra::Matrix4;

use crate::components::{Transform, TransformMatrix};
use hecs::{PreparedQuery, World};

pub fn update_transform_matrix_system(
    query: &mut PreparedQuery<(&Transform, &mut TransformMatrix)>,
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
    use crate::components::Transform;
    use approx::assert_relative_eq;
    use nalgebra::{vector, UnitQuaternion};

    use super::*;

    #[test]
    pub fn test_update_transform_matrix() {
        let mut world = World::new();
        let transform = Transform::default();
        let transform_matrix = TransformMatrix::default();

        let entity = world.spawn((transform, transform_matrix));

        {
            let matrix = world.get_mut::<TransformMatrix>(entity).unwrap();
            assert_eq!(matrix.0, Matrix4::identity());
        }

        let test_translation = vector![5.0, 1.0, 2.0];
        let test_rotation = UnitQuaternion::from_euler_angles(0.3, 0.3, 0.3);

        {
            let mut transform = world.get_mut::<Transform>(entity).unwrap();
            transform.translation = test_translation.clone();
            transform.rotation = test_rotation.clone();
            transform.scale = test_translation.clone();
        }

        update_transform_matrix_system(&mut Default::default(), &mut world);

        let expected_matrix = Matrix4::new_translation(&test_translation)
            * Matrix4::from(test_rotation)
            * Matrix4::new_nonuniform_scaling(&vector![5.0, 1.0, 2.0]);

        let matrix = world.get_mut::<TransformMatrix>(entity).unwrap();
        assert_relative_eq!(matrix.0, expected_matrix);
    }
}
