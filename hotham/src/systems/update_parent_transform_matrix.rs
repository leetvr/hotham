use std::collections::HashMap;

use crate::components::{Info, Parent, Root, TransformMatrix};
use itertools::Itertools;
use legion::{component, system, world::SubWorld, Entity, EntityStore, IntoQuery};
use nalgebra::Matrix4;

#[system]
#[write_component(TransformMatrix)]
#[read_component(Parent)]
#[read_component(Info)]
#[read_component(Root)]
pub fn update_parent_transform_matrix(world: &mut SubWorld) -> () {
    // Build heirarchy.
    let mut heirarchy: HashMap<Entity, Vec<Entity>> = HashMap::new();
    let mut query = <&Parent>::query();
    query.for_each_chunk(world, |chunk| {
        chunk.into_iter_entities().for_each(|(entity, parent)| {
            let children = heirarchy.entry(parent.0).or_default();
            children.push(entity);
        });
    });

    let mut query = Entity::query().filter(!component::<Parent>() & component::<TransformMatrix>());
    let roots = query.iter(world).map(|e| e.clone()).collect_vec();
    for root in &roots {
        let root_entry = world.entry_ref(*root).unwrap();
        let root_matrix = root_entry.get_component::<TransformMatrix>().unwrap().0;
        update_transform_matrix(root_matrix, *root, &heirarchy, world);
    }
}

fn update_transform_matrix(
    parent_matrix: Matrix4<f32>,
    entity: Entity,
    heirarchy: &HashMap<Entity, Vec<Entity>>,
    world: &mut SubWorld,
) {
    if let Some(children) = heirarchy.get(&entity) {
        for child in children {
            let mut child_entry = world.entry_mut(*child).unwrap();
            let mut child_matrix = child_entry.get_component::<TransformMatrix>().unwrap().0;
            child_matrix = parent_matrix * child_matrix;
            child_entry
                .get_component_mut::<TransformMatrix>()
                .unwrap()
                .0 = child_matrix;
            update_transform_matrix(child_matrix, *child, heirarchy, world);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use approx::{assert_relative_eq, relative_eq};
    use legion::{Entity, Resources, Schedule, World};
    use nalgebra::vector;

    use crate::{
        components::{Info, Transform},
        systems::update_transform_matrix_system,
    };

    use super::*;
    #[test]
    pub fn test_transform_system() {
        let (mut world, mut schedule, mut resources) = setup();
        let parent_transform_matrix =
            TransformMatrix(Matrix4::new_translation(&vector![1.0, 1.0, 100.0]));

        let parent = world.push((parent_transform_matrix.clone(),));
        let child = world.push((parent_transform_matrix.clone(), Parent(parent)));
        let grandchild = world.push((parent_transform_matrix.clone(), Parent(child)));

        schedule.execute(&mut world, &mut resources);

        let grandchild = world.entry(grandchild).unwrap();
        let transform_matrix = grandchild.get_component::<TransformMatrix>().unwrap();
        let expected_matrix = Matrix4::new_translation(&vector![3.0, 3.0, 300.0]);
        assert_relative_eq!(transform_matrix.0, expected_matrix);

        let child = world.entry(child).unwrap();
        let transform_matrix = child.get_component::<TransformMatrix>().unwrap();
        let expected_matrix = Matrix4::new_translation(&vector![2.0, 2.0, 200.0]);
        assert_relative_eq!(transform_matrix.0, expected_matrix);
    }

    #[test]
    pub fn test_transform_system_extensive() {
        let (mut world, mut schedule, mut resources) = setup();
        let mut heirachy: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut node_entity: HashMap<usize, Entity> = HashMap::new();
        let mut entity_node: HashMap<Entity, usize> = HashMap::new();
        heirachy.insert(0, vec![1, 2, 3, 4]);
        heirachy.insert(1, vec![5, 6, 7, 8]);
        heirachy.insert(2, vec![9, 10, 11, 12]);
        heirachy.insert(3, vec![13, 14, 15, 16]);
        heirachy.insert(5, vec![17, 18, 19, 20]);
        heirachy.insert(14, vec![21, 22, 23, 24]);
        heirachy.insert(22, vec![25, 26, 27, 28]);
        heirachy.insert(17, vec![29, 30, 31, 32]);

        for n in 0..=32 {
            let info = Info {
                name: format!("Node {}", n),
                node_id: n,
            };
            let mut transform = Transform::default();
            transform.translation = vector![1.0, 1.0, 1.0];
            let matrix = TransformMatrix::default();
            let entity = world.push((info, transform, matrix));
            node_entity.insert(n, entity);
            entity_node.insert(entity, n);
        }

        for (parent, children) in heirachy.iter() {
            let parent_entity = node_entity.get(parent).unwrap().clone();
            let parent = Parent(parent_entity);
            for node_id in children {
                let entity = node_entity.get(node_id).unwrap();
                let mut child = world.entry(*entity).unwrap();
                child.add_component(parent.clone());
            }
        }

        let root_entity = node_entity.get(&0).unwrap();
        let mut root = world.entry(*root_entity).unwrap();
        let transform = root.get_component_mut::<Transform>().unwrap();
        transform.translation = vector![100.0, 100.0, 100.0];
        schedule.execute(&mut world, &mut resources);

        let mut query = <(&TransformMatrix, &Parent, &Info)>::query();
        for (transform_matrix, parent, info) in query.iter(&world) {
            let mut depth = 1;

            let mut parent_entity = parent.0;
            let mut parent_matrices = vec![];
            loop {
                let parent = world.entry_ref(parent_entity).unwrap();
                let parent_transform_matrix = parent.get_component::<TransformMatrix>().unwrap();
                parent_matrices.push(parent_transform_matrix.0);
                // Walk up the tree until we find the root.
                if let Ok(grand_parent) = parent.get_component::<Parent>() {
                    depth += 1;
                    parent_entity = grand_parent.0;
                } else {
                    let expected_matrix = get_expected_matrix(depth);
                    if !relative_eq!(expected_matrix, transform_matrix.0) {
                        panic!(
                            "[Node {}] - {:?} did not equal {:?} at depth {}",
                            info.node_id, transform_matrix.0, expected_matrix, depth
                        );
                    }
                    break;
                }
            }
        }
    }

    fn get_expected_matrix(depth: usize) -> Matrix4<f32> {
        let mut transform = Matrix4::new_translation(&vector![100.0, 100.0, 100.0]);
        for _ in 0..depth {
            transform = transform * Matrix4::new_translation(&vector![1.0, 1.0, 1.0]);
        }
        transform
    }

    #[test]
    pub fn test_entities_without_transforms() {
        let (mut world, mut schedule, mut resources) = setup();
        world.push((0,));

        schedule.execute(&mut world, &mut resources);
    }

    fn setup() -> (World, Schedule, Resources) {
        let schedule = Schedule::builder()
            .add_system(update_transform_matrix_system())
            .add_system(update_parent_transform_matrix_system())
            .build();

        (Default::default(), schedule, Default::default())
    }
}
