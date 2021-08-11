use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

use cgmath::{vec3, Matrix4, Quaternion, Transform, Vector3};

use crate::{
    animation::Animation, buffer::Buffer, components::Mesh, resources::VulkanContext, skin::Skin,
};
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
    pub mesh: Option<Mesh>,
    pub skin: Option<Skin>,
    pub animations: Vec<Rc<RefCell<Animation>>>,
    pub active_animation_index: Option<usize>,
}

impl Node {
    pub(crate) fn load(
        node_data: &gltf::Node,
        buffers: &Vec<&[u8]>,
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
        skin_buffer: &Buffer<Matrix4<f32>>,
        parent: Weak<RefCell<Node>>,
    ) -> Result<(String, Rc<RefCell<Node>>)> {
        let name = node_data.name().unwrap().to_string();
        let is_root = parent.upgrade().is_none();
        println!("Loading node {} - {}", node_data.index(), name);

        let mesh = if let Some(mesh_data) = node_data.mesh() {
            Some(Mesh::load(
                &mesh_data,
                buffers,
                vulkan_context,
                set_layouts[0],
                skin_buffer,
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
            rotation: Quaternion::new(rotation[3], rotation[0], rotation[1], rotation[2]), // gltf gives a quaternion in [x, y, z, w], but we need [w, x, y, z]
            mesh,
            skin: None,
            animations: Default::default(),
            active_animation_index: None,
        };

        let node = Rc::new(RefCell::new(node));
        let parent_node = Rc::downgrade(&node);

        // let children = (*node).borrow().get_children(
        //     buffers,
        //     vulkan_context,
        //     set_layouts,
        //     ubo_buffer,
        //     node_data,
        //     parent_node,
        // )?;
        // (*node).borrow_mut().children = children;

        // Special case: the root node has no parent, so it must apply its own skin.
        if is_root {
            if let Some(skin_data) = node_data.skin() {
                Skin::load(
                    &skin_data,
                    buffers,
                    vulkan_context,
                    node.clone(),
                    set_layouts[1],
                )?;
            }

            // NOTE: This *must* be done after load_children as skins will refer to children that may not be loaded yet.
            load_child_skins(
                buffers,
                vulkan_context,
                node_data,
                node.clone(),
                set_layouts[1],
            )?;
        }

        Ok((name, node))
    }

    // PERF: This algorithm is terrible.
    pub fn find(&self, index: usize) -> Option<Rc<RefCell<Node>>> {
        // Go up to the top of the tree
        if let Some(parent) = self.parent.upgrade() {
            // Check that this isn't the node we're after.
            if (*parent).borrow().index == index {
                return Some(parent);
            }

            return (*parent).borrow().find(index);
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
            if (**child).borrow().index == index {
                return Some(child.clone());
            } else {
                node = (**child).borrow().find_node_in_child(index);
            }

            if node.is_some() {
                break;
            }
        }

        node
    }

    pub(crate) fn update_joints(&self, vulkan_context: &VulkanContext) -> Result<()> {
        let node_matrix = self.get_node_matrix();
        if let Some(skin) = self.skin.as_ref() {
            let inverse_transform = node_matrix.inverse_transform().unwrap();
            let joints = &skin.joints;
            let inverse_bind_matrices = &skin.inverse_bind_matrices;

            let joint_matrices = joints
                .iter()
                .zip(inverse_bind_matrices)
                .map(|(joint, inverse_bind_matrix)| {
                    let joint = (**joint).borrow();
                    let result = inverse_transform * joint.get_node_matrix() * inverse_bind_matrix;
                    result
                })
                .collect::<Vec<_>>();

            skin.ssbo.update(
                vulkan_context,
                joint_matrices.as_ptr(),
                joint_matrices.len(),
            )?;
        }

        for child in &self.children {
            let child = (**child).borrow();
            child.update_joints(vulkan_context)?;
        }

        Ok(())
    }

    pub fn get_node_matrix(&self) -> Matrix4<f32> {
        let mut node_matrix = self.get_local_matrix();
        let mut parent = self.parent.clone();

        // Walk up the tree to the root
        while let Some(p) = parent.upgrade() {
            let p = (*p).borrow();
            node_matrix = p.get_local_matrix() * node_matrix;
            parent = p.parent.clone();
        }

        node_matrix
    }

    pub fn get_local_matrix(&self) -> Matrix4<f32> {
        let translation = Matrix4::from_translation(self.translation);
        let rotation = Matrix4::from(self.rotation);
        let scale = Matrix4::from_nonuniform_scale(self.scale.x, self.scale.y, self.scale.z);

        return translation * rotation * scale;
    }

    pub fn get_root_node(&self) -> Option<Rc<RefCell<Node>>> {
        let mut last_parent = None;
        let mut parent = self.parent.clone();

        // Walk up the tree to the root
        while let Some(p) = parent.upgrade() {
            last_parent = Some(p.clone());
            let p = (*p).borrow();
            parent = p.parent.clone();
        }

        last_parent
    }

    pub(crate) fn _update_animation_to_percentage(
        &self,
        percentage: f32,
        vulkan_context: &VulkanContext,
    ) -> Result<()> {
        if let Some(index) = self.active_animation_index {
            let animation = self.animations.get(index).ok_or_else(|| {
                anyhow!(
                    "Unable to find animation with index {} on node {}",
                    index,
                    self.index
                )
            })?;
            (**animation).borrow_mut()._update_to_percentage(percentage);
            self.update_joints(vulkan_context)?;
        }
        Ok(())
    }

    pub(crate) fn _update_animation(
        &self,
        delta_time: f32,
        vulkan_context: &VulkanContext,
    ) -> Result<()> {
        if let Some(index) = self.active_animation_index {
            let animation = self.animations.get(index).ok_or_else(|| {
                anyhow!(
                    "Unable to find animation with index {} on node {}",
                    index,
                    self.index
                )
            })?;
            (**animation).borrow_mut()._update(delta_time);
            self.update_joints(vulkan_context)?;
        }
        Ok(())
    }

    // fn get_children(
    //     &self,
    //     buffers: &Vec<&[u8]>,
    //     vulkan_context: &VulkanContext,
    //     set_layouts: &[vk::DescriptorSetLayout],
    //     ubo_buffer: vk::Buffer,
    //     node_data: &gltf::Node,
    //     parent_node: Weak<RefCell<Node>>,
    // ) -> Result<Vec<Rc<RefCell<Node>>>> {
    //     node_data
    //         .children()
    //         .map(|n| {
    //             let (_, node) = Node::load(
    //                 &n,
    //                 buffers,
    //                 vulkan_context,
    //                 set_layouts,
    //                 ubo_buffer,
    //                 parent_node.clone(),
    //             )?;
    //             Ok(node)
    //         })
    //         .collect::<Result<Vec<_>>>()
    // }
}

fn load_child_skins(
    buffers: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    node_data: &gltf::Node,
    skeleton_root: Rc<RefCell<Node>>,
    skin_descriptor_set_layout: vk::DescriptorSetLayout,
) -> Result<()> {
    // If our children need skins, create them.
    for child in node_data.children() {
        let skeleton_root = skeleton_root.clone();
        if let Some(skin_data) = child.skin() {
            let index = child.index();
            println!(
                "{} - {} needs a skin, loading..",
                index,
                child.name().unwrap()
            );
            let child_node = (*skeleton_root)
                .borrow()
                .find(index)
                .ok_or_else(|| anyhow!("Child {} not found on skeleton root", index))?;
            Skin::load(
                &skin_data,
                buffers,
                vulkan_context,
                child_node,
                skin_descriptor_set_layout,
            )?;
        }

        load_child_skins(
            buffers,
            vulkan_context,
            &child,
            skeleton_root,
            skin_descriptor_set_layout,
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use cgmath::SquareMatrix;

    use super::*;
    #[test]
    pub fn test_node_matrix() {
        let parent = Node {
            index: 0,
            parent: Weak::new(),
            children: Vec::new(),
            translation: vec3(0.0, 0.0, 0.0),
            scale: vec3(1.0, 1.0, 1.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            mesh: None,
            skin: None,
            animations: Vec::new(),
            active_animation_index: None,
        };

        let parent_ref = Rc::new(RefCell::new(parent));

        let child = Node {
            index: 1,
            parent: Rc::downgrade(&parent_ref),
            children: Vec::new(),
            translation: vec3(0.0, 0.0, 0.0),
            scale: vec3(1.0, 1.0, 1.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            mesh: None,
            skin: None,
            animations: Vec::new(),
            active_animation_index: None,
        };

        let child_ref = Rc::new(RefCell::new(child));
        (*parent_ref).borrow_mut().children.push(child_ref.clone());

        let grandchild = Node {
            index: 2,
            parent: Rc::downgrade(&child_ref),
            children: Vec::new(),
            translation: vec3(0.0, 0.0, 0.0),
            scale: vec3(1.0, 1.0, 1.0),
            rotation: Quaternion::new(0.0, 0.0, 0.0, 0.0),
            mesh: None,
            skin: None,
            animations: Vec::new(),
            active_animation_index: None,
        };

        let grandchild_ref = Rc::new(RefCell::new(grandchild));
        (*child_ref)
            .borrow_mut()
            .children
            .push(grandchild_ref.clone());

        let grandchild = parent_ref.borrow().find(2).unwrap();
        assert_eq!(grandchild.borrow().get_node_matrix(), Matrix4::identity());

        (*parent_ref).borrow_mut().translation = vec3(1.0, 2.0, 3.0);
        let expected_translation = Matrix4::from_translation(vec3(1.0, 2.0, 3.0));
        assert_eq!(grandchild.borrow().get_node_matrix(), expected_translation);

        (*child_ref).borrow_mut().translation = vec3(-1.0, 0.0, 0.0);
        let expected_translation = Matrix4::from_translation(vec3(1.0, 2.0, 3.0))
            * Matrix4::from_translation(vec3(-1.0, 0.0, 0.0));
        assert_eq!(grandchild.borrow().get_node_matrix(), expected_translation);

        (*grandchild_ref).borrow_mut().translation = vec3(1.0, 0.0, 0.0);
        let expected_translation = Matrix4::from_translation(vec3(1.0, 2.0, 3.0))
            * Matrix4::from_translation(vec3(-1.0, 0.0, 0.0))
            * Matrix4::from_translation(vec3(1.0, 0.0, 0.0));
        assert_eq!(grandchild.borrow().get_node_matrix(), expected_translation);

        // TODO: Persist upon cloning
        // let cloned_parent = Node::clone(&parent_ref.borrow());
        // drop(parent_ref);

        // let parent_ref = Rc::new(RefCell::new(cloned_parent));
        // (*parent_ref).borrow_mut().translation = vec3(5.0, 4.0, 3.0);

        // let child_ref = Rc::clone(
        //     (*parent_ref)
        //         .borrow()
        //         .children
        //         .iter()
        //         .next()
        //         .as_ref()
        //         .unwrap(),
        // );
        // let expected_translation = Matrix4::from_translation(vec3(5.0, 4.0, 3.0));
        // assert_eq!(child_ref.borrow().get_node_matrix(), expected_translation);
    }
}
