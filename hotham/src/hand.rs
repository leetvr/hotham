use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::{animation::Animation, node::Node, vulkan_context::VulkanContext};

pub(crate) struct Hand {
    node: Rc<RefCell<Node>>,
    default_animation: Rc<RefCell<Animation>>,
    grip_animation: Rc<RefCell<Animation>>,
}

impl Hand {
    pub(crate) fn new(node: Rc<RefCell<Node>>) -> Self {
        let n = (*node).borrow();
        assert_eq!(n.animations.len(), 2, "Node must have two animations!");
        let default_animation = n.animations[0].clone();
        let grip_animation = n.animations[1].clone();
        drop(n);

        Self {
            node,
            default_animation,
            grip_animation,
        }
    }

    pub(crate) fn grip(&self, percentage: f32, vulkan_context: &VulkanContext) -> Result<()> {
        {
            let grip_animation = (*self.grip_animation).borrow();
            (*self.default_animation)
                .borrow()
                .blend(&grip_animation, percentage)?;
        }

        (*self.node).borrow().update_joints(vulkan_context)
    }
}

#[cfg(test)]
mod tests {
    use cgmath::Matrix4;

    use crate::{
        gltf_loader, renderer::create_descriptor_set_layouts, vulkan_context::VulkanContext,
    };
    use ash::vk;

    use super::*;

    #[test]
    pub fn grip_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let hand_node = get_hand_node(&vulkan_context);
        let hand = Hand::new(hand_node);
        let before = get_joint_matrices(&hand);
        hand.grip(0.5, &vulkan_context).unwrap();
        let after = get_joint_matrices(&hand);
        assert_ne!(before, after);
    }

    fn get_joint_matrices(hand: &Hand) -> Vec<Matrix4<f32>> {
        Default::default()
    }

    fn get_hand_node(vulkan_context: &VulkanContext) -> Rc<RefCell<Node>> {
        let (document, buffers, _) = gltf::import("../test_assets/left_hand.gltf").unwrap();
        let set_layouts = create_descriptor_set_layouts(vulkan_context).unwrap();
        let ubo_buffer = vk::Buffer::null();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();

        let gltf_bytes = document.into_json().to_vec().unwrap();
        let mut nodes = gltf_loader::load_gltf_nodes(
            &gltf_bytes,
            &buffers,
            &vulkan_context,
            &set_layouts,
            ubo_buffer,
        )
        .unwrap();

        let hand = nodes.drain().next().unwrap().1;
        hand
    }
}
