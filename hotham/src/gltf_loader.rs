use crate::{
    buffer::Buffer, mesh::Mesh, node::Node, texture::Texture, vulkan_context::VulkanContext, Vertex,
};
use anyhow::{anyhow, Result};
use cgmath::{vec2, vec3, vec4, Matrix4, Quaternion};
use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    path::{Path, PathBuf},
    rc::Rc,
    sync::{Arc, Mutex},
};

use ash::vk;

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
        let (name, node) = load_node(&node_data, blob, vulkan_context, set_layouts, ubo_buffer);
        nodes.insert(name, node);
    }

    Ok(nodes)
}

fn load_node(
    node_data: &gltf::Node,
    blob: &[u8],
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
    ubo_buffer: vk::Buffer,
) -> (String, Node) {
    let name = node_data.name().unwrap().to_string();
    let mesh = node_data
        .mesh()
        .map(|m| {
            Mesh::load(&m, blob, vulkan_context, set_layouts, ubo_buffer)
                .map_err(|e| eprintln!("Error loading mesh {:?}", e))
                .ok()
        })
        .flatten();
    let transform = node_data.transform();
    let (translation, rotation, scale) = transform.clone().decomposed();
    let children = node_data
        .children()
        .map(|n| {
            let (_, node) = load_node(&n, blob, vulkan_context, set_layouts, ubo_buffer);
            RefCell::new(Rc::new(node))
        })
        .collect::<Vec<_>>();

    let node = Node {
        parent: None,
        children,
        translation: vec3(translation[0], translation[1], translation[2]),
        scale: vec3(scale[0], scale[1], scale[2]),
        rotation: Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3]),
        skin_index: node_data.skin().map(|s| s.index()),
        matrix: Matrix4::from(transform.matrix()),
        mesh,
    };

    (name, node)
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
        assert!(hand.children.len() > 1);
    }
}
