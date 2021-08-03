use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use anyhow::Result;
use cgmath::{vec3, Deg, Euler, Quaternion, Vector3};

use crate::{animation::Animation, node::Node, vulkan_context::VulkanContext};

#[derive(Clone, Debug)]
pub(crate) struct Hand {
    parent_node_inner: Rc<RefCell<Node>>,
    root_bone_node_inner: Rc<RefCell<Node>>,
    default_animation: Rc<RefCell<Animation>>,
    grip_animation: Rc<RefCell<Animation>>,
}

impl Hand {
    pub(crate) fn new(node: Rc<RefCell<Node>>) -> Self {
        let n = (*node).borrow();
        assert_eq!(n.animations.len(), 2, "Node must have two animations!");
        let default_animation = n.animations[0].clone();
        let grip_animation = n.animations[1].clone();
        let root_bone_node_inner = n.find(3).expect("Unable to find root bone node");
        drop(n);

        Self {
            parent_node_inner: node,
            root_bone_node_inner,
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

        (&self.node()).update_joints(vulkan_context)
    }

    pub(crate) fn update_position(
        &self,
        translation: Vector3<f32>,
        rotation: Quaternion<f32>,
    ) -> () {
        let mut root_bone_node = (*self.root_bone_node_inner).borrow_mut();
        root_bone_node.translation = translation + vec3(0.0075, -0.005, -0.0525);
        let mut rotation = Euler::from(rotation);
        rotation.x += Deg(40.0).into();
        root_bone_node.rotation = Quaternion::from(rotation);
    }

    pub(crate) fn node<'a>(&'a self) -> Ref<'a, Node> {
        (*self.parent_node_inner).borrow()
    }

    pub(crate) fn _node_mut<'a>(&'a self) -> RefMut<'a, Node> {
        (*self.parent_node_inner).borrow_mut()
    }
}

#[cfg(test)]
mod tests {
    use cgmath::{vec3, Deg, Euler, Matrix4, Quaternion, Rad, Rotation3};

    use crate::{
        gltf_loader, renderer::create_descriptor_set_layouts, vulkan_context::VulkanContext,
    };
    use ash::{version::DeviceV1_0, vk};

    use super::*;

    #[test]
    pub fn grip_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let hand_node = get_hand_node(&vulkan_context);
        let hand = Hand::new(hand_node);
        let before = get_joint_matrices(&hand, &vulkan_context);
        hand.grip(0.5, &vulkan_context).unwrap();
        let after = get_joint_matrices(&hand, &vulkan_context);
        assert_ne!(before, after);
    }

    #[test]
    pub fn move_test() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let hand_node = get_hand_node(&vulkan_context);
        let hand = Hand::new(hand_node);
        let new_position = vec3(1.0, 0.0, 0.0);
        let new_rotation = Quaternion::from_angle_x(Deg(90.0));
        println!("{:?}", new_rotation);
        hand.update_position(new_position, new_rotation);

        let root_bone_node = (*hand.root_bone_node_inner).borrow();
        let expected_rotation = Rad(3.1415925);
        assert_eq!(root_bone_node.translation, new_position);
        assert_eq!(Euler::from(root_bone_node.rotation).x, expected_rotation);
    }

    fn get_joint_matrices(hand: &Hand, vulkan_context: &VulkanContext) -> Vec<Matrix4<f32>> {
        let hand = (*hand).parent_node_inner.borrow().find(2).unwrap();
        let hand = (*hand).borrow();
        let skin = hand.skin.as_ref().unwrap();
        let vertex_buffer = &skin.ssbo;
        let matrices: &[Matrix4<f32>];

        unsafe {
            let memory = vulkan_context
                .device
                .map_memory(
                    vertex_buffer.device_memory,
                    0,
                    vk::WHOLE_SIZE,
                    vk::MemoryMapFlags::empty(),
                )
                .unwrap();
            matrices = std::slice::from_raw_parts_mut(std::mem::transmute(memory), 25);
            assert_eq!(matrices.len(), 25);
        }

        matrices.to_vec()
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
