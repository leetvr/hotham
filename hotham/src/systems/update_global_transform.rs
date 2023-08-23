use crate::{
    components::{GlobalTransform, LocalTransform, Parent},
    Engine,
};
use hecs::World;

/// Update global transform system
/// Updates [`GlobalTransform`] based on [`LocalTransform`] and the hierarchy of [`Parent`]s.
pub fn update_global_transform_system(engine: &mut Engine) {
    let world = &mut engine.world;
    update_global_transform_system_inner(world);
}

pub(crate) fn update_global_transform_system_inner(world: &mut World) {
    // Update GlobalTransform of roots
    for (_, (local_transform, global_transform)) in world
        .query_mut::<(&LocalTransform, &mut GlobalTransform)>()
        .without::<&Parent>()
    {
        global_transform.0 = local_transform.to_affine();
    }

    // Construct a view for efficient random access into the set of all entities that have
    // parents. Views allow work like dynamic borrow checking or component storage look-up to be
    // done once rather than per-entity as in `World::get`.
    let mut parents = world.query::<(&Parent, &LocalTransform)>();
    let parents = parents.view();

    // View of entities that don't have parents, i.e. roots of the transform hierarchy
    let mut roots = world.query::<&GlobalTransform>().without::<&Parent>();
    let roots = roots.view();

    // This query can coexist with the `roots` view without illegal aliasing of `GlobalTransform`
    // references because the inclusion of `&Parent` in the query, and its exclusion from the view,
    // guarantees that they will never overlap. Similarly, it can coexist with `parents` because
    // that view does not reference `GlobalTransform`s at all.
    for (_entity, (parent, local_transform, global_transform)) in world
        .query::<(&Parent, &LocalTransform, &mut GlobalTransform)>()
        .iter()
    {
        // Walk the hierarchy from this entity to the root, accumulating the entity's absolute
        // transform. This does a small amount of redundant work for intermediate levels of deeper
        // hierarchies, but unlike a top-down traversal, avoids tracking entity child lists and is
        // cache-friendly.
        let mut relative = local_transform.to_affine();
        let mut ancestor = parent.0;
        while let Some((next, next_local)) = parents.get(ancestor) {
            relative = next_local.to_affine() * relative;
            ancestor = next.0;
        }
        // The `while` loop terminates when `ancestor` cannot be found in `parents`, i.e. when it
        // does not have a `Parent` component, and is therefore necessarily a root.
        global_transform.0 = roots.get(ancestor).unwrap().0 * relative;
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use approx::{assert_relative_eq, relative_eq};
    use glam::{Affine3A, EulerRot, Quat};
    use hecs::Entity;

    use crate::components::Info;

    use super::*;

    #[test]
    pub fn test_transform_system() {
        let mut world = World::new();
        let parent_local_transform = LocalTransform {
            translation: [1.0, 1.0, 100.0].into(),
            ..Default::default()
        };
        let parent_global_transform =
            GlobalTransform(Affine3A::from_translation([1.0, 1.0, 100.0].into()));

        let parent = world.spawn((parent_local_transform, parent_global_transform));
        let child = world.spawn((
            parent_local_transform,
            parent_global_transform,
            Parent(parent),
        ));
        let grandchild = world.spawn((
            parent_local_transform,
            parent_global_transform,
            Parent(child),
        ));

        update_global_transform_system_inner(&mut world);

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
        update_global_transform_system_inner(&mut world);

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
        update_global_transform_system_inner(&mut world);
    }

    #[test]
    pub fn test_update_global_transform_system() {
        let mut world = World::new();
        let local_transform = LocalTransform::default();
        let global_transform = GlobalTransform::default();

        let entity = world.spawn((local_transform, global_transform));

        {
            let matrix = world.get::<&mut GlobalTransform>(entity).unwrap();
            assert_eq!(matrix.0, Affine3A::IDENTITY);
        }

        let test_translation = [5.0, 1.0, 2.0].into();
        let test_rotation = Quat::from_euler(EulerRot::XYZ, 0.3, 0.3, 0.3);

        {
            let mut local_transform = world.get::<&mut LocalTransform>(entity).unwrap();
            local_transform.translation = test_translation;
            local_transform.rotation = test_rotation;
            local_transform.scale = test_translation;
        }

        update_global_transform_system_inner(&mut world);

        let expected_matrix = Affine3A::from_scale_rotation_translation(
            [5.0, 1.0, 2.0].into(),
            test_rotation,
            test_translation,
        );

        let global_transform = world.get::<&mut GlobalTransform>(entity).unwrap();
        assert_relative_eq!(global_transform.0, expected_matrix);
    }
}
