use nalgebra::Matrix4;

use crate::asset_importer::ImportContext;

/// Component added to an entity to point to the joints in the node
/// Automatically added by `gltf_loader`
#[derive(Debug, Clone, PartialEq)]
pub struct Skin {
    /// List of joints, represented by their node ID
    pub joint_ids: Vec<usize>,
}

impl Skin {
    pub(crate) fn load(skin: gltf::Skin, import_context: &mut ImportContext) {
        // let this_entity = *node_entity_map.get(&node_data.index()).unwrap();
        let mut joint_matrices = Vec::new();
        let reader = skin.reader(|_| Some(&import_context.buffer));
        let matrices = reader.read_inverse_bind_matrices().unwrap();
        for m in matrices {
            let m = Matrix4::from(m);
            joint_matrices.push(m);
        }
        // let mut joint_ids = Vec::new();

        // for (joint_node, inverse_bind_matrix) in node_skin_data.joints().zip(joint_matrices.iter()) {
        //     let joint = Joint {
        //         skeleton_root: this_entity,
        //         inverse_bind_matrix: *inverse_bind_matrix,
        //     };
        //     joint_ids.push(joint_node.index());
        //     let joint_entity = node_entity_map.get(&joint_node.index()).unwrap();
        //     world.insert_one(*joint_entity, joint).unwrap();
        // }

        // // Add a Skin to the entity.
        // world.insert_one(this_entity, Skin { joint_ids }).unwrap();

        // // Tell the vertex shader how many joints we have
        // let mut mesh = world.get_mut::<Mesh>(this_entity).unwrap();
        // mesh.ubo_data.joint_count = joint_matrices.len() as f32;

        // for child in node_data.children() {
        //     add_skins_and_joints(&child, buffer, world, vulkan_context, node_entity_map);
        // }
    }
}
