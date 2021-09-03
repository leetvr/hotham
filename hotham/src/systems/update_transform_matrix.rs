use legion::system;
use nalgebra::Matrix4;

use crate::components::{Transform, TransformMatrix};

#[system(for_each)]
pub fn update_transform_matrix(transform: &Transform, transform_matrix: &mut TransformMatrix) {
    transform_matrix.0 = Matrix4::new_translation(&transform.translation)
        * Matrix4::from(transform.rotation)
        * Matrix4::new_nonuniform_scaling(&transform.scale);
}

#[cfg(test)]
mod tests {
    use crate::components::Transform;
    use approx::assert_relative_eq;
    use legion::{EntityStore, Resources, Schedule, World};
    use nalgebra::{vector, UnitQuaternion};

    use super::*;

    #[test]
    pub fn test_update_transform_matrix() {
        let mut world = World::default();
        let mut resources = Resources::default();
        let transform = Transform::default();
        let transform_matrix = TransformMatrix::default();

        let entity = world.push((transform, transform_matrix));
        let mut scheduler = Schedule::builder()
            .add_system(update_transform_matrix_system())
            .build();
        scheduler.execute(&mut world, &mut resources);

        let mut e = world.entry_mut(entity).unwrap();
        let matrix = e.get_component_mut::<TransformMatrix>().unwrap();
        assert_eq!(matrix.0, Matrix4::identity());

        let test_translation = vector![5.0, 1.0, 2.0];
        let test_rotation = UnitQuaternion::from_euler_angles(0.3, 0.3, 0.3);

        let transform = e.get_component_mut::<Transform>().unwrap();
        transform.translation = test_translation.clone();
        transform.rotation = test_rotation.clone();
        transform.scale = test_translation.clone();
        drop(e);

        scheduler.execute(&mut world, &mut resources);

        let expected_matrix = Matrix4::new_translation(&test_translation)
            * Matrix4::from(test_rotation)
            * Matrix4::new_nonuniform_scaling(&vector![5.0, 1.0, 2.0]);

        let e = world.entry(entity).unwrap();
        let matrix = e.get_component::<TransformMatrix>().unwrap();
        assert_relative_eq!(matrix.0, expected_matrix);
    }
}
