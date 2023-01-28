use std::collections::HashMap;

use crate::{
    components::{GlobalTransform, Parent},
    Engine,
};
use glam::Affine3A;
use hecs::{Entity, Without, World};

/// Update global transform with parent transform system
/// Walks through each entity that has a Parent and builds a hierarchy
/// Then transforms each entity based on the hierarchy
pub fn update_global_transform_with_parent_system(engine: &mut Engine) {
    let world = &mut engine.world;
    update_global_transform_with_parent_system_inner(world);
}

pub(crate) fn update_global_transform_with_parent_system_inner(world: &mut World) {
    // Build hierarchy
    let mut hierarchy: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for (entity, parent) in world.query_mut::<&Parent>() {
        let children = hierarchy.entry(parent.0).or_default();
        children.push(entity);
    }

    let mut roots = world.query::<Without<&GlobalTransform, &Parent>>();
    for (root, root_matrix) in roots.iter() {
        update_global_transforms_recursively(&root_matrix.0, root, &hierarchy, world);
    }
}

fn update_global_transforms_recursively(
    parent_matrix: &Affine3A,
    entity: Entity,
    hierarchy: &HashMap<Entity, Vec<Entity>>,
    world: &World,
) {
    if let Some(children) = hierarchy.get(&entity) {
        for child in children {
            {
                let child_matrix = &mut world.get::<&mut GlobalTransform>(*child).unwrap().0;
                *child_matrix = *parent_matrix * *child_matrix;
            }
            let child_matrix = world.get::<&GlobalTransform>(*child).unwrap().0;
            update_global_transforms_recursively(&child_matrix, *child, hierarchy, world);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use approx::{assert_relative_eq, relative_eq};
    use glam::Affine3A;

    use crate::{
        components::{Info, LocalTransform},
        systems::update_global_transform::update_global_transform_system_inner,
    };

    use super::*;
    #[test]
    pub fn test_transform_system() {
        let mut world = World::new();
        let parent_global_transform =
            GlobalTransform(Affine3A::from_translation([1.0, 1.0, 100.0].into()));

        let parent = world.spawn((parent_global_transform,));
        let child = world.spawn((parent_global_transform, Parent(parent)));
        let grandchild = world.spawn((parent_global_transform, Parent(child)));

        tick(&mut world);

        {
            let global_transform = world.get::<&GlobalTransform>(grandchild).unwrap();
            let expected_matrix = Affine3A::from_translation([3.0, 3.0, 300.0].into());
            assert_relative_eq!(global_transform.0, expected_matrix);
        }

        {
            let global_transform = world.get::<&GlobalTransform>(child).unwrap();
            let expected_matrix = Affine3A::from_translation([2.0, 2.0, 200.0].into());
            assert_relative_eq!(global_transform.0, expected_matrix);
        }
    }

    #[test]
    pub fn test_transform_system_extensive() {
        let mut world = World::new();
        let mut hierarchy: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut node_entity: HashMap<usize, Entity> = HashMap::new();
        let mut entity_node: HashMap<Entity, usize> = HashMap::new();
        hierarchy.insert(0, vec![1, 2, 3, 4]);
        hierarchy.insert(1, vec![5, 6, 7, 8]);
        hierarchy.insert(2, vec![9, 10, 11, 12]);
        hierarchy.insert(3, vec![13, 14, 15, 16]);
        hierarchy.insert(5, vec![17, 18, 19, 20]);
        hierarchy.insert(14, vec![21, 22, 23, 24]);
        hierarchy.insert(22, vec![25, 26, 27, 28]);
        hierarchy.insert(17, vec![29, 30, 31, 32]);

        for n in 0..=32 {
            let info = Info {
                name: format!("Node {n}"),
                node_id: n,
            };
            let local_transform = LocalTransform {
                translation: [1.0, 1.0, 1.0].into(),
                ..Default::default()
            };
            let matrix = GlobalTransform::default();
            let entity = world.spawn((info, local_transform, matrix));
            node_entity.insert(n, entity);
            entity_node.insert(entity, n);
        }

        for (parent, children) in hierarchy.iter() {
            let parent_entity = node_entity.get(parent).unwrap();
            let parent = Parent(*parent_entity);
            for node_id in children {
                let entity = node_entity.get(node_id).unwrap();
                world.insert_one(*entity, parent).unwrap();
            }
        }

        let root_entity = node_entity.get(&0).unwrap();
        {
            let mut local_transform = world.get::<&mut LocalTransform>(*root_entity).unwrap();
            local_transform.translation = [100.0, 100.0, 100.0].into();
        }
        tick(&mut world);

        for (_, (global_transform, parent, info)) in
            world.query::<(&GlobalTransform, &Parent, &Info)>().iter()
        {
            let mut depth = 1;

            let mut parent_entity = parent.0;
            let mut parent_matrices = vec![];
            loop {
                let parent_global_transform = world.get::<&GlobalTransform>(parent_entity).unwrap();
                parent_matrices.push(parent_global_transform.0);

                // Walk up the tree until we find the root.
                if let Ok(grand_parent) = world.get::<&Parent>(parent_entity) {
                    depth += 1;
                    parent_entity = grand_parent.0;
                } else {
                    let expected_matrix = get_expected_matrix(depth);
                    if !relative_eq!(expected_matrix, global_transform.0) {
                        panic!(
                            "[Node {}] - {:?} did not equal {:?} at depth {}",
                            info.node_id, global_transform.0, expected_matrix, depth
                        );
                    }
                    break;
                }
            }
        }
    }

    fn get_expected_matrix(depth: usize) -> Affine3A {
        let mut transform = Affine3A::from_translation([100.0, 100.0, 100.0].into());
        for _ in 0..depth {
            transform = transform * Affine3A::from_translation([1.0, 1.0, 1.0].into());
        }
        transform
    }

    #[test]
    pub fn test_entities_without_transforms() {
        let mut world = World::new();
        world.spawn((0,));
        tick(&mut world);
    }

    fn tick(world: &mut World) {
        update_global_transform_system_inner(world);
        update_global_transform_with_parent_system_inner(world);
    }
}
