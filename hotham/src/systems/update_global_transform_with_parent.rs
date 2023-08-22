use crate::{
    components::{GlobalTransform, Parent},
    Engine,
};
use hecs::World;

/// Update global transform with parent transform system
/// Walks through each entity that has a Parent and builds a hierarchy
/// Then transforms each entity based on the hierarchy
pub fn update_global_transform_with_parent_system(engine: &mut Engine) {
    let world = &mut engine.world;
    update_global_transform_with_parent_system_inner(world);
}

pub(crate) fn update_global_transform_with_parent_system_inner(world: &mut World) {
    // Construct a view for efficient random access into the set of all entities that have
    // parents. Views allow work like dynamic borrow checking or component storage look-up to be
    // done once rather than per-entity as in `World::get`.
    let mut parents = world.query::<&Parent>();
    let parents = parents.view();

    // View of entities that don't have parents, i.e. roots of the transform hierarchy
    let mut roots = world.query::<&GlobalTransform>().without::<&Parent>();
    let roots = roots.view();

    // This query can coexist with the `roots` view without illegal aliasing of `GlobalTransform`
    // references because the inclusion of `&Parent` in the query, and its exclusion from the view,
    // guarantees that they will never overlap. Similarly, it can coexist with `parents` because
    // that view does not reference `GlobalTransform`s at all.
    for (_entity, (parent, absolute)) in world.query::<(&Parent, &mut GlobalTransform)>().iter() {
        // Walk the hierarchy from this entity to the root, accumulating the entity's absolute
        // transform. This does a small amount of redundant work for intermediate levels of deeper
        // hierarchies, but unlike a top-down traversal, avoids tracking entity child lists and is
        // cache-friendly.
        let mut relative = parent.from_child;
        let mut ancestor = parent.entity;
        while let Some(next) = parents.get(ancestor) {
            relative = next.from_child * relative;
            ancestor = next.entity;
        }
        // The `while` loop terminates when `ancestor` cannot be found in `parents`, i.e. when it
        // does not have a `Parent` component, and is therefore necessarily a root.
        absolute.0 = roots.get(ancestor).unwrap().0 * relative;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use approx::{assert_relative_eq, relative_eq};
    use glam::Affine3A;
    use hecs::Entity;

    use crate::{
        components::Info, systems::update_global_transform::update_global_transform_system_inner,
    };

    use super::*;
    #[test]
    pub fn test_transform_system() {
        let mut world = World::new();
        let global_transform =
            GlobalTransform(Affine3A::from_translation([1.0, 1.0, 100.0].into()));
        let from_child = Affine3A::from_translation([1.0, 1.0, 100.0].into());

        let parent = world.spawn((global_transform,));
        let child = world.spawn((
            global_transform,
            Parent {
                entity: parent,
                from_child,
            },
        ));
        let grandchild = world.spawn((
            global_transform,
            Parent {
                entity: child,
                from_child,
            },
        ));

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
            let global_transform =
                GlobalTransform(Affine3A::from_translation([1.0, 1.0, 1.0].into()));
            let entity = world.spawn((info, global_transform));
            node_entity.insert(n, entity);
            entity_node.insert(entity, n);
        }

        for (parent, children) in hierarchy.iter() {
            let parent_entity = node_entity.get(parent).unwrap();
            let parent = Parent {
                entity: *parent_entity,
                from_child: Affine3A::from_translation([1.0, 1.0, 1.0].into()),
            };
            for node_id in children {
                let entity = node_entity.get(node_id).unwrap();
                world.insert_one(*entity, parent).unwrap();
            }
        }

        let root_entity = node_entity.get(&0).unwrap();
        {
            let mut global_transform = world.get::<&mut GlobalTransform>(*root_entity).unwrap();
            global_transform.0.translation = [100.0, 100.0, 100.0].into();
        }
        tick(&mut world);

        for (_, (global_transform, parent, info)) in
            world.query::<(&GlobalTransform, &Parent, &Info)>().iter()
        {
            let mut depth = 1;

            let mut parent_entity = parent.entity;
            let mut parent_matrices = vec![];
            loop {
                let parent_global_transform = world.get::<&GlobalTransform>(parent_entity).unwrap();
                parent_matrices.push(parent_global_transform.0);

                // Walk up the tree until we find the root.
                if let Ok(grand_parent) = world.get::<&Parent>(parent_entity) {
                    depth += 1;
                    parent_entity = grand_parent.entity;
                } else {
                    let expected_matrix = get_expected_matrix(depth);
                    if !relative_eq!(expected_matrix, global_transform.0) {
                        panic!(
                            "[Node {}] - {:?} did not equal {expected_matrix:?} at depth {depth}",
                            info.node_id, global_transform.0
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
