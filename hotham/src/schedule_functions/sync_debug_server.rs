use std::collections::HashMap;

use hotham_debug_server::{
    debug_data::{DebugData, DebugEntity, DebugTransform},
    DebugServer,
};
use legion::{EntityStore, IntoQuery, Resources, World};

use crate::{
    components::{transform, Collider, Info, Material, Mesh, Transform},
    util::entity_to_u64,
};

pub fn sync_debug_server(world: &mut World, resources: &mut Resources) {
    let mut debug_server = resources.get_mut::<DebugServer>().unwrap();
    let frame = resources.get::<usize>().unwrap();
    let debug_data = world_to_debug_data(&world, *frame);

    let _ = debug_server.sync(&debug_data);
}

// TODO: We should really just be serializing the whole world here, but whatever.
pub fn world_to_debug_data(world: &World, frame: usize) -> DebugData {
    let mut entities = HashMap::new();
    let mut query = <&Info>::query();
    query.for_each_chunk(world, |c| {
        for (entity, info) in c.into_iter_entities() {
            let entry = world.entry_ref(entity).unwrap();
            let transform = entry.get_component::<Transform>().ok();
            let collider = entry.get_component::<Collider>().ok();

            let e = DebugEntity {
                name: info.name.clone(),
                id: entity_to_u64(entity),
                transform: transform.map(parse_transform),
                collider: None,
            };

            entities.insert(e.id, e);
        }
    });
    return DebugData {
        id: frame as _,
        entities,
    };
}

fn parse_transform(transform: &Transform) -> DebugTransform {
    let t = transform.translation;
    let r = transform.rotation.euler_angles();
    let s = transform.scale;

    return DebugTransform {
        translation: [t[0], t[1], t[2]],
        rotation: [r.0, r.1, r.2],
        scale: [s[0], s[1], s[2]],
    };
}
