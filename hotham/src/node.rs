use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use cgmath::{vec3, Matrix4, Quaternion, Vector3};

use crate::skin::Skin;
use crate::{mesh::Mesh, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;

#[derive(Debug, Clone)]
pub struct Node {
    pub index: usize,
    pub parent: Weak<RefCell<Node>>,
    pub children: Vec<Rc<RefCell<Node>>>,
    pub translation: Vector3<f32>,
    pub scale: Vector3<f32>,
    pub rotation: Quaternion<f32>,
    pub matrix: Matrix4<f32>,
    pub mesh: Option<Mesh>,
    pub skin: Option<Skin>,
}

impl Node {
    pub(crate) fn load(
        node_data: &gltf::Node,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
        ubo_buffer: vk::Buffer,
        parent: Weak<RefCell<Node>>,
    ) -> Result<(String, Rc<RefCell<Node>>)> {
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

        let node = Node {
            index: node_data.index(),
            parent,
            children: Default::default(),
            translation: vec3(translation[0], translation[1], translation[2]),
            scale: vec3(scale[0], scale[1], scale[2]),
            rotation: Quaternion::new(rotation[0], rotation[1], rotation[2], rotation[3]),
            matrix: Matrix4::from(transform.matrix()),
            mesh,
            skin: None,
        };

        let node = Rc::new(RefCell::new(node));

        if let Some(s) = node_data.skin() {
            let skin = Skin::load(&s, blob, vulkan_context, set_layouts, node.clone())?;
            node.borrow_mut().skin.replace(skin);
        }

        node_data
            .children()
            .map(|n| {
                let parent_node = Rc::downgrade(&node);
                let (_, child_node) = Node::load(
                    &n,
                    blob,
                    vulkan_context,
                    set_layouts,
                    ubo_buffer,
                    parent_node,
                )?;
                node.borrow_mut().children.push(child_node);

                Ok(())
            })
            .collect::<Result<_>>()?;

        Ok((name, node))
    }
}
