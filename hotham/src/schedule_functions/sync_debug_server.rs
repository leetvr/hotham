use hecs::World;
use hotham_debug_server::{
    debug_frame::{DebugCollider, DebugEntity, DebugFrame, DebugTransform},
    DebugServer,
};
use uuid::Uuid;

use crate::{
    components::{Collider, Info, Transform},
    resources::PhysicsContext,
};

pub fn sync_debug_server(
    world: &mut World,
    debug_server: &mut DebugServer,
    physics_context: &PhysicsContext,
) {
    let debug_data = world_to_debug_data(
        &world,
        &physics_context,
        debug_server.current_frame,
        debug_server.session_id,
    );
    debug_server.frame_queue.push(debug_data);

    if debug_server.time_since_last_sync() > 1 {
        debug_server.sync();
    }

    debug_server.current_frame += 1; // TODO: We should really have a frame counter elsewhere..
}

// TODO: We should really just be serializing the whole world here, but whatever.
pub fn world_to_debug_data(
    world: &World,
    physics_context: &PhysicsContext,
    frame: usize,
    session_id: Uuid,
) -> DebugFrame {
    let mut entities = Vec::new();
    for (entity, info) in world.query::<&Info>().iter() {
        let transform = world.get_mut::<&Transform>(entity).ok();
        let collider = world.get_mut::<&Collider>(entity).ok();
        let collider = collider
            .map(|c| physics_context.colliders.get(c.handle))
            .flatten();
        let entity_id = entity.id() as _;

        let e = DebugEntity {
            name: info.name.clone(),
            id: format!("{}_{}", session_id, entity_id),
            entity_id,
            transform: transform.map(|t| parse_transform(*t)),
            collider: collider.map(parse_collider),
        };

        entities.push(e);
    }
    return DebugFrame {
        id: Uuid::new_v4(),
        frame_number: frame as _,
        entities,
        session_id,
    };
}

fn parse_transform(transform: &Transform) -> DebugTransform {
    let t = transform.translation;
    let r = transform.rotation.quaternion();
    let s = transform.scale;

    return DebugTransform {
        translation: [t[0], t[1], t[2]],
        rotation: [r[0], r[1], r[2], r[3]],
        scale: [s[0], s[1], s[2]],
    };
}

fn parse_collider(collider: &rapier3d::geometry::Collider) -> DebugCollider {
    let shape_type = collider.shape().shape_type();
    let collider_type = match shape_type {
        rapier3d::prelude::ShapeType::Cuboid => "cube",
        rapier3d::prelude::ShapeType::Cylinder => "cylinder",
        _ => panic!("Incompatible shape {:?} found", shape_type),
    };

    let geometry = if shape_type == rapier3d::prelude::ShapeType::Cuboid {
        let cube = collider.shape().as_cuboid().unwrap();
        let h = cube.half_extents;
        vec![h[0], h[1], h[2]]
    } else {
        let cylinder = collider.shape().as_cylinder().unwrap();
        vec![cylinder.half_height, cylinder.radius]
    };

    let t = collider.translation();
    let translation = [t[0], t[1], t[2]];
    let r = collider.rotation().quaternion();

    DebugCollider {
        collider_type: collider_type.to_string(),
        geometry,
        translation,
        rotation: [r[0], r[1], r[2], r[3]],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nalgebra::{vector, UnitQuaternion};
    use rapier3d::prelude::{ColliderBuilder, RigidBodyBuilder};

    use crate::{
        components::{Info, Transform},
        resources::PhysicsContext,
    };

    #[test]
    fn test_world_to_debug() {
        let mut world = World::new();
        let mut physics_context = PhysicsContext::default();

        let e1 = world.spawn((
            Info {
                name: "Test".to_string(),
                node_id: 2,
            },
            Transform {
                translation: vector![1., 2., 3.],
                scale: vector![3., 2., 1.],
                rotation: UnitQuaternion::from_euler_angles(0., 0., 0.),
            },
        ));

        let rigid_body = RigidBodyBuilder::new_dynamic().build();
        let collider = ColliderBuilder::cuboid(1.0, 1.0, 1.0)
            .translation(vector![0., 0.5, 0.])
            .build();
        let components = physics_context.get_rigid_body_and_collider(e1, rigid_body, collider);
        {
            world.insert(e1, components).unwrap();
        }

        let e2 = world.spawn((
            Info {
                name: "Test 2".to_string(),
                node_id: 3,
            },
            Transform {
                translation: vector![4., 5., 6.],
                scale: vector![6., 5., 4.],
                rotation: UnitQuaternion::from_euler_angles(0., 0., 0.),
            },
        ));

        let rigid_body = RigidBodyBuilder::new_dynamic().build();
        let collider = ColliderBuilder::cylinder(1.0, 0.2).build();
        let components = physics_context.get_rigid_body_and_collider(e2, rigid_body, collider);
        {
            world.insert(e2, components).unwrap();
        }

        let session_id = Uuid::new_v4();
        let debug_data = world_to_debug_data(&world, &physics_context, 666, session_id);
        assert_eq!(debug_data.frame_number, 666);

        let e1 = e1.id() as u64;
        let e2 = e2.id() as u64;

        let debug_entity1 = debug_data
            .entities
            .iter()
            .find(|&e| e.entity_id == e1)
            .unwrap();
        assert_eq!(debug_entity1.name, "Test".to_string());
        assert_eq!(debug_entity1.entity_id, e1);
        assert_eq!(
            debug_entity1.transform,
            Some(DebugTransform {
                translation: [1., 2., 3.],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [3., 2., 1.]
            })
        );
        assert_eq!(
            debug_entity1.collider,
            Some(DebugCollider {
                collider_type: "cube".to_string(),
                geometry: vec![1., 1., 1.,],
                translation: [0., 0.5, 0.],
                rotation: [0.0, 0.0, 0.0, 1.0],
            })
        );

        let debug_entity2 = debug_data
            .entities
            .iter()
            .find(|&e| e.entity_id == e2)
            .unwrap();
        assert_eq!(debug_entity2.name, "Test 2".to_string());
        assert_eq!(debug_entity2.entity_id, e2);
        assert_eq!(
            debug_entity2.transform,
            Some(DebugTransform {
                translation: [4., 5., 6.],
                rotation: [0.0, 0.0, 0.0, 1.0],
                scale: [6., 5., 4.]
            })
        );
    }
}
