use std::{cell::RefCell, rc::Rc};

use anyhow::Result;

use crate::node::Node;

pub(crate) struct Hand {
    node: Rc<RefCell<Node>>,
}

impl Hand {
    pub(crate) fn new(node: Rc<RefCell<Node>>) -> Self {
        Self { node }
    }

    pub(crate) fn grip(&self, amount: f32) -> Result<()> {
        Ok(())
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
        let hand_node = get_hand_node();
        let hand = Hand::new(hand_node);
        let before = get_joint_matrices(&hand);
        hand.grip(0.5).unwrap();
        let after = get_joint_matrices(&hand);
        assert_ne!(before, after);
    }

    fn get_joint_matrices(hand: &Hand) -> Vec<Matrix4<f32>> {
        Default::default()
    }

    fn get_hand_node() -> Rc<RefCell<Node>> {
        let (document, buffers, _) = gltf::import("../test_assets/left_hand.gltf").unwrap();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();
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

        nodes.into_iter().next().unwrap().1
    }
}
