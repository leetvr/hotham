use crate::{animation::Animation, node::Node, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    rc::{Rc, Weak},
};

pub(crate) fn load_gltf_nodes(
    gltf_bytes: &[u8],
    data_bytes: &[u8],
    vulkan_context: &VulkanContext,
    mesh_descriptor_set_layout: vk::DescriptorSetLayout,
    ubo_buffer: vk::Buffer,
) -> Result<HashMap<String, Rc<RefCell<Node>>>> {
    let gtlf_buf = Cursor::new(gltf_bytes);
    let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
    let document = gltf.document;
    let blob = data_bytes;

    let mut nodes = HashMap::new();
    let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene

    for node_data in root_scene.nodes() {
        let (name, node) = Node::load(
            &node_data,
            blob,
            vulkan_context,
            &[mesh_descriptor_set_layout],
            ubo_buffer,
            Weak::new(),
        )?;
        (*node).borrow().update_joints(vulkan_context)?;
        nodes.insert(name, node);
    }

    let nodes_vec = nodes.values().collect::<Vec<_>>();

    for animation in document.animations() {
        Animation::load(&animation, blob, &nodes_vec)?;
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{renderer::create_descriptor_set_layouts, vulkan_context::VulkanContext};
    #[test]
    pub fn test_asteroid() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let gltf = include_bytes!("../../hotham-asteroid/assets/asteroid.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/asteroid_data.bin");
        let buffer = vk::Buffer::null();
        let nodes = load_gltf_nodes(gltf, data, &vulkan_context, set_layouts[0], buffer).unwrap();
        assert!(nodes.len() != 0);
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let gltf = document.into_json().to_vec().unwrap();
        let data = &buffers[0];
        let buffer = vk::Buffer::null();
        let nodes = load_gltf_nodes(&gltf, data, &vulkan_context, set_layouts[0], buffer).unwrap();
        assert!(nodes.len() == 1);

        let hand = nodes.get("Hand").unwrap();

        let children = &hand.borrow().children;
        assert!(children.len() == 2);

        let palm = children.get(1).unwrap();
        assert!(palm.borrow().children.len() == 5);
        assert!(Rc::ptr_eq(&palm.borrow().parent.upgrade().unwrap(), hand));

        let hand_base = children.get(0).unwrap().borrow();
        assert!(Rc::ptr_eq(&hand_base.parent.upgrade().unwrap(), hand));

        let skin = hand_base.skin.as_ref().unwrap();
        assert_eq!(skin.inverse_bind_matrices.len(), 16);
        assert_eq!(skin.joints.len(), 16);
    }
}
