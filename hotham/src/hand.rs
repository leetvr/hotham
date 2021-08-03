use std::{
    cell::{Ref, RefCell, RefMut},
    rc::Rc,
};

use anyhow::Result;
use cgmath::{vec3, Deg, Euler, Quaternion, Rotation, Rotation3, Vector3};

use crate::{
    animation::Animation, node::Node, util::to_euler_degrees, vulkan_context::VulkanContext,
};

#[derive(Clone, Debug)]
pub(crate) struct Hand {
    parent_node_inner: Rc<RefCell<Node>>,
    root_bone_node_inner: Rc<RefCell<Node>>,
    default_animation: Rc<RefCell<Animation>>,
    grip_animation: Rc<RefCell<Animation>>,
    grip_offset: (Vector3<f32>, Quaternion<f32>),
}

impl Hand {
    pub(crate) fn new(node: Rc<RefCell<Node>>) -> Self {
        let n = (*node).borrow();
        assert_eq!(n.animations.len(), 2, "Node must have two animations!");
        let default_animation = n.animations[0].clone();
        let grip_animation = n.animations[1].clone();
        let root_bone_node_inner = n.find(3).expect("Unable to find root bone node");
        let hand_node_inner = n.find(4).expect("Unable to find hand node");
        let hand_node = hand_node_inner.borrow();
        let grip_node = n.find(5).expect("Unable to find grip node");
        let grip_node = grip_node.borrow();
        let root_bone_node = root_bone_node_inner.borrow();

        let root_rotation = to_euler_degrees(root_bone_node.rotation);
        let grip_rotation = to_euler_degrees(grip_node.rotation);
        let hand_rotation = to_euler_degrees(hand_node.rotation);

        let mut offset_rotation = Euler {
            x: Deg(00.0),
            y: Deg(00.0),
            z: Deg(0.0),
        };
        // offset_rotation.y = offset_rotation.y - Deg(24.0);

        let grip_offset = (
            hand_node.translation + grip_node.translation,
            offset_rotation.into(),
        );
        println!(
            "Root: {:?} - Grip: {:?} - Hand: {:?}",
            root_rotation, grip_rotation, hand_rotation
        );
        println!("Offset: {:?}", to_euler_degrees(grip_offset.1));
        drop(n);
        drop(root_bone_node);

        Self {
            parent_node_inner: node,
            root_bone_node_inner,
            default_animation,
            grip_animation,
            grip_offset,
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
        let (translation_offset, rotation_offset) = &self.grip_offset;
        // let (translation_offset, rotation_offset) =
        //     (vec3(0.0, 0.0, 0.0), Quaternion::new(0.0, 0.0, 0.0, 0.0));
        root_bone_node.translation = translation - translation_offset;
        root_bone_node.rotation = rotation * rotation_offset;
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
    use cgmath::{assert_relative_eq, vec3, Deg, Euler, Matrix4, Quaternion, Rad, Rotation3};

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
        let new_rotation = Quaternion::from_angle_y(Deg(90.0));
        println!("{:?}", new_rotation);
        hand.update_position(new_position, new_rotation);

        let root_bone_node = (*hand.root_bone_node_inner).borrow();
        let expected_rotation = Quaternion::from_angle_y(Deg(150.0));
        // assert_relative_eq!(root_bone_node.translation, expected_translation);
        assert_relative_eq!(
            Euler::from(root_bone_node.rotation),
            expected_rotation.into()
        );
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
