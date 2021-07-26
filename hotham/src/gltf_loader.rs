use crate::{node::Node, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;
use std::{collections::HashMap, io::Cursor};

pub(crate) fn load_gltf_nodes(
    gltf_bytes: &[u8],
    data_bytes: &[u8],
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
    ubo_buffer: vk::Buffer,
) -> Result<HashMap<String, Node>> {
    let gtlf_buf = Cursor::new(gltf_bytes);
    let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
    let document = gltf.document;
    let blob = data_bytes;

    let mut nodes = HashMap::new();
    let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene

    for node_data in root_scene.nodes() {
        let (name, node) = Node::load(&node_data, blob, vulkan_context, set_layouts, ubo_buffer)?;
        nodes.insert(name, node);
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{renderer::create_descriptor_set_layout, vulkan_context::VulkanContext};
    #[test]
    pub fn test_asteroid() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let gltf = include_bytes!("../../hotham-asteroid/assets/asteroid.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/asteroid_data.bin");
        let set_layout = create_descriptor_set_layout(&vulkan_context).unwrap();
        let buffer = vk::Buffer::null();
        let nodes = load_gltf_nodes(gltf, data, &vulkan_context, &[set_layout], buffer).unwrap();
        assert!(nodes.len() != 0);
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let gltf = document.into_json().to_vec().unwrap();
        let data = &buffers[0];
        let set_layout = create_descriptor_set_layout(&vulkan_context).unwrap();
        let buffer = vk::Buffer::null();
        let nodes = load_gltf_nodes(&gltf, data, &vulkan_context, &[set_layout], buffer).unwrap();
        assert!(nodes.len() == 1);
        let hand = nodes.get("Hand").unwrap();
        let palm = hand.children.get(0).unwrap();
        assert!(palm.borrow().children.len() == 5);
        assert_eq!(palm.borrow().parent.upgrade().unwrap().index, hand.index);
    }
}
