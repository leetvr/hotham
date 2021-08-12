use crate::components::{Parent, TransformMatrix};
use legion::{system, world::SubWorld, EntityStore, IntoQuery};

#[system]
#[write_component(TransformMatrix)]
#[read_component(Parent)]
pub fn update_parent_transform_matrix(world: &mut SubWorld) -> () {
    unsafe {
        let mut query = <(&mut TransformMatrix, &Parent)>::query();
        query.for_each_unchecked(world, |(transform_matrix, parent)| {
            let parent = world.entry_ref(parent.0).unwrap();
            let parent_matrix = parent.get_component::<TransformMatrix>().unwrap().0;

            transform_matrix.0 = parent_matrix * transform_matrix.0;
        });
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{assert_relative_eq, vec3, Matrix4, SquareMatrix};
    use legion::{Schedule, World};

    use super::*;
    #[test]
    pub fn test_transform_system() {
        let mut world = World::default();
        let parent_transform_matrix =
            TransformMatrix(Matrix4::from_translation(vec3(1.0, 1.0, 1.0)));

        let parent = world.push((parent_transform_matrix.clone(),));
        let child = world.push((parent_transform_matrix.clone(), Parent(parent)));
        let grandchild = world.push((parent_transform_matrix.clone(), Parent(child)));

        let mut schedule = Schedule::builder()
            .add_system(update_parent_transform_matrix_system())
            .build();
        let mut resources = Default::default();
        schedule.execute(&mut world, &mut resources);

        let grandchild = world.entry(grandchild).unwrap();
        let transform_matrix = grandchild.get_component::<TransformMatrix>().unwrap();
        let expected_matrix = get_expected_matrix(3);
        assert_relative_eq!(transform_matrix.0, expected_matrix);

        let child = world.entry(child).unwrap();
        let transform_matrix = child.get_component::<TransformMatrix>().unwrap();
        let expected_matrix = get_expected_matrix(2);
        assert_relative_eq!(transform_matrix.0, expected_matrix);
    }

    fn get_expected_matrix(depth: usize) -> Matrix4<f32> {
        let mut transform = Matrix4::identity();
        for _ in 0..depth {
            transform = transform * Matrix4::from_translation(vec3(1.0, 1.0, 1.0));
        }
        transform
    }
}
