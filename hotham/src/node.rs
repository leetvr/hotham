use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use cgmath::{vec3, Matrix4, Quaternion, Vector3};

use crate::{animation::Animation, mesh::Mesh, skin::Skin, vulkan_context::VulkanContext};
use anyhow::{anyhow, Result};
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
    pub animations: Vec<Animation>,
    pub active_animation_index: Option<usize>,
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
        println!("Loading node {} - {}", node_data.index(), name);

        let mesh = if let Some(mesh_data) = node_data.mesh() {
            Some(Mesh::load(
                &mesh_data,
                blob,
                vulkan_context,
                set_layouts,
                ubo_buffer,
            )?)
        } else {
            None
        };

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
            animations: Default::default(),
            active_animation_index: None,
        };

        let node = Rc::new(RefCell::new(node));
        let parent_node = Rc::downgrade(&node);

        (*node).borrow_mut().load_children(
            blob,
            vulkan_context,
            set_layouts,
            ubo_buffer,
            node_data,
            parent_node,
        )?;

        // NOTE: This *must* be done after load_children as skins will refer to children that may not be loaded yet.
        (*node)
            .borrow()
            .load_skins(blob, vulkan_context, node_data, node.clone())?;

        Ok((name, node))
    }

    // PERF: This algorithm is terrible.
    pub fn find(&self, index: usize) -> Option<Rc<RefCell<Node>>> {
        // Go up to the top of the tree
        if let Some(parent) = self.parent.upgrade() {
            // Check that this isn't the node we're after.
            if parent.borrow().index == index {
                return Some(parent);
            }

            return parent.borrow().find(index);
        }

        // We are at the top of the tree.
        self.find_node_in_child(index)
    }

    pub fn find_node_in_child(&self, index: usize) -> Option<Rc<RefCell<Node>>> {
        if self.children.is_empty() {
            return None;
        }

        let mut node = None;

        for child in &self.children {
            if child.borrow().index == index {
                return Some(child.clone());
            } else {
                node = child.borrow().find_node_in_child(index);
            }

            if node.is_some() {
                break;
            }
        }

        node
    }

    fn load_children(
        &mut self,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
        ubo_buffer: vk::Buffer,
        node_data: &gltf::Node,
        parent_node: Weak<RefCell<Node>>,
    ) -> Result<()> {
        self.children = node_data
            .children()
            .map(|n| {
                let (_, node) = Node::load(
                    &n,
                    blob,
                    vulkan_context,
                    set_layouts,
                    ubo_buffer,
                    parent_node.clone(),
                )?;
                Ok(node)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(())
    }

    fn load_skins(
        &self,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        node_data: &gltf::Node,
        skeleton_root: Rc<RefCell<Node>>,
    ) -> Result<()> {
        // If our children need skins, create them.
        for child in node_data.children() {
            if let Some(skin_data) = child.skin() {
                let index = child.index();
                println!(
                    "{} - {} needs a skin, loading..",
                    index,
                    child.name().unwrap()
                );
                let child_node = skeleton_root
                    .borrow()
                    .find(index)
                    .ok_or_else(|| anyhow!("Child not found"))?;
                Skin::load(&skin_data, blob, vulkan_context, child_node)?;
            }
        }

        Ok(())
    }
}
