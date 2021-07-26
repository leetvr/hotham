use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use cgmath::{vec3, Matrix4, Quaternion, Vector3};

use crate::{mesh::Mesh, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;

#[derive(Debug, Clone)]
pub struct Node {
    pub index: usize,
    pub parent: Weak<Node>,
    pub children: Vec<RefCell<Rc<Node>>>,
    pub translation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub skin_index: Option<usize>,
    pub matrix: Matrix4<f32>,
    pub mesh: Option<Mesh>,
}

impl Node {
    pub(crate) fn load(
        node_data: &gltf::Node,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
        ubo_buffer: vk::Buffer,
    ) -> Result<(String, Node)> {
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
                let (_, node) = Node::load(&n, blob, vulkan_context, set_layouts, ubo_buffer)?;
                Ok(RefCell::new(Rc::new(node)))
            })
            .collect::<Result<Vec<_>>>()?;

        let node = Node {
            index: node_data.index(),
            parent: Default::default(),
            children,
            translation: vec3(translation[0], translation[1], translation[2]),
            scale: vec3(scale[0], scale[1], scale[2]),
            rotation: Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3]),
            skin_index: node_data.skin().map(|s| s.index()),
            matrix: Matrix4::from(transform.matrix()),
            mesh,
        };

        Ok((name, node))
    }
}
