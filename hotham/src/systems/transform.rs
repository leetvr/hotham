use cgmath::Matrix4;
use legion::{system, world::SubWorld, EntityStore, IntoQuery};

use crate::components::transform::Transform;

#[system]
#[write_component(Transform)]
pub fn transform(world: &mut SubWorld) -> () {
    let mut query = <(&mut Transform,)>::query();
    unsafe {
        query.for_each_unchecked(world, |t| {
            let (mut transform,) = t;

            transform.local_matrix = get_local_matrix(transform);
            transform.global_matrix = get_global_matrix(transform, world);
        })
    }
}

pub(crate) fn get_global_matrix<E: EntityStore>(transform: &Transform, world: &E) -> Matrix4<f32> {
    if let Some(parent) = transform.parent.as_ref() {
        let parent = world.entry_ref(*parent).unwrap();
        let parent_transform = parent.get_component::<Transform>().unwrap();
        parent_transform.global_matrix * transform.local_matrix
    } else {
        transform.local_matrix
    }
}

pub(crate) fn get_local_matrix(transform: &Transform) -> Matrix4<f32> {
    let local_matrix = Matrix4::from_translation(transform.translation)
        * Matrix4::from(transform.rotation)
        * Matrix4::from_nonuniform_scale(transform.scale.x, transform.scale.y, transform.scale.z);
    local_matrix
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, vec3, Matrix4, SquareMatrix, Vector3};
    use legion::{Schedule, World};

    use super::*;
    #[test]
    pub fn test_transform_system() {
        let mut world = World::default();
        let parent_transform = Transform {
            translation: Vector3::new(1.0, 1.0, 1.0),
            ..Default::default()
        };
        let parent = world.push((parent_transform.clone(),));
        let mut child_transform = parent_transform.clone();
        child_transform.parent = Some(parent);
        let child = world.push((child_transform,));

        let mut grandchild_transform = parent_transform.clone();
        grandchild_transform.parent = Some(child);
        let grandchild = world.push((grandchild_transform,));

        let mut schedule = Schedule::builder().add_system(transform_system()).build();
        let mut resources = Default::default();
        schedule.execute(&mut world, &mut resources);
        let expected_local_matrix = get_expected_matrix(1);

        let grandchild = world.entry(grandchild).unwrap();
        let transform = grandchild.get_component::<Transform>().unwrap();
        let expected_matrix = get_expected_matrix(3);
        assert_relative_eq!(transform.global_matrix, expected_matrix);
        assert_relative_eq!(transform.local_matrix, expected_local_matrix);

        let child = world.entry(child).unwrap();
        let transform = child.get_component::<Transform>().unwrap();
        let expected_matrix = get_expected_matrix(2);
        assert_relative_eq!(transform.global_matrix, expected_matrix);
        assert_relative_eq!(transform.local_matrix, expected_local_matrix);

        let parent_transform = Matrix4::from_translation(vec3(1.0, 0.0, 0.0));
        let child_local_transform = Matrix4::from_translation(vec3(1.0, 0.0, 0.0));
        let child_global_transform = parent_transform * child_local_transform;
        assert_relative_eq!(
            child_global_transform,
            Matrix4::from_translation(vec3(2.0, 0.0, 0.0))
        );

        let undo = parent_transform.invert().unwrap() * child_global_transform;
        assert_relative_eq!(undo, child_local_transform);
    }

    fn get_expected_matrix(depth: usize) -> Matrix4<f32> {
        let mut transform = Matrix4::identity();
        for _ in 0..depth {
            transform = transform * Matrix4::from_translation(vec3(1.0, 1.0, 1.0));
        }
        transform
    }
}
