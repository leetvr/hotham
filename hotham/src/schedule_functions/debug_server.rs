use std::collections::HashMap;

use hotham_debug_server::{
    debug_data::{DebugData, DebugEntity},
    DebugServer,
};
use legion::{EntityStore, IntoQuery, Resources, World};

use crate::{
    components::{Collider, Info, Material, Mesh, Transform},
    util::entity_to_u64,
};

pub fn debug_server(world: &mut World, resources: &mut Resources) {
    let mut debug_server = resources.get_mut::<DebugServer>().unwrap();
}

// TODO: We should really just be serializing the whole world here, but whatever.
pub fn world_to_debug_data(world: &World, frame: usize) -> DebugData {
    let mut entities = HashMap::new();
    let mut query = <&Info>::query();
    query.for_each_chunk(world, |c| {
        for (entity, info) in c.into_iter_entities() {
            let entry = world.entry_ref(entity).unwrap();
            let transform = entry.get_component::<Transform>();
            let collider = entry.get_component::<Collider>();
            let material = entry.get_component::<Material>();
            let mesh = entry.get_component::<Mesh>();

            let e = DebugEntity {
                name: info.name.clone(),
                id: entity_to_u64(entity),
                mesh: None,
                material: None,
                transform: None,
                collider: None,
            };
        }
    });
    return DebugData {
        id: frame as _,
        entities,
    };
}
