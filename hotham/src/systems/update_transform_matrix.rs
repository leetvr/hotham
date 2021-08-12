use cgmath::Matrix4;
use legion::system;

use crate::components::{Transform, TransformMatrix};

#[system(for_each)]
pub fn update_transform_matrix(transform: &Transform, transform_matrix: &mut TransformMatrix) {
    transform_matrix.0 = Matrix4::from_translation(transform.translation)
        * Matrix4::from(transform.rotation)
        * Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
}

#[cfg(test)]
mod tests {
    use crate::components::Transform;
    use cgmath::{assert_relative_eq, vec3, Quaternion, SquareMatrix};
    use legion::{EntityStore, Resources, Schedule, World};

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

        let test_translation = vec3(5.0, 1.0, 2.0);
        let test_rotation = Quaternion::new(0.3, 0.3, 0.3, 0.3);

        let transform = e.get_component_mut::<Transform>().unwrap();
        transform.translation = test_translation.clone();
        transform.rotation = test_rotation.clone();
        transform.scale = test_translation.clone();
        drop(e);

        scheduler.execute(&mut world, &mut resources);

        let expected_matrix = Matrix4::from_translation(test_translation.clone())
            * Matrix4::from(test_rotation)
            * Matrix4::from_nonuniform_scale(5.0, 1.0, 2.0);

        let e = world.entry(entity).unwrap();
        let matrix = e.get_component::<TransformMatrix>().unwrap();
        assert_relative_eq!(matrix.0, expected_matrix);
    }
}
