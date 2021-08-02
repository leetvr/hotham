use crate::{animation::Animation, node::Node, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;
use std::{
    cell::RefCell,
    collections::HashMap,
    io::Cursor,
    rc::{Rc, Weak},
};

pub(crate) fn load_gltf_nodes(
    gltf_bytes: &[u8],
    buffers: &Vec<&[u8]>,
    vulkan_context: &VulkanContext,
    descriptor_set_layouts: &[vk::DescriptorSetLayout],
    ubo_buffer: vk::Buffer,
) -> Result<HashMap<String, Rc<RefCell<Node>>>> {
    let gtlf_buf = Cursor::new(gltf_bytes);
    let gltf = gltf::Gltf::from_reader(gtlf_buf)?;
    let document = gltf.document;
    let blob = buffers;

    let mut nodes = HashMap::new();
    let root_scene = document.scenes().next().unwrap(); // safe as there is always one scene

    for node_data in root_scene.nodes() {
        let (name, node) = Node::load(
            &node_data,
            buffers,
            vulkan_context,
            descriptor_set_layouts,
            ubo_buffer,
            Weak::new(),
        )?;
        (*node).borrow().update_joints(vulkan_context)?;
        nodes.insert(name, node);
    }

    let nodes_vec = nodes.values().collect::<Vec<_>>();

    for animation in document.animations() {
        Animation::load(&animation, blob, &nodes_vec)?;
    }

    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use ash::version::DeviceV1_0;
    use cgmath::{vec3, vec4, Matrix4, Quaternion};

    use super::*;
    use crate::{renderer::create_descriptor_set_layouts, vulkan_context::VulkanContext, Vertex};
    #[test]
    pub fn test_asteroid() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let gltf = include_bytes!("../../hotham-asteroid/assets/asteroid.gltf");
        let data = include_bytes!("../../hotham-asteroid/assets/asteroid_data.bin").to_vec();
        let buffer = vk::Buffer::null();
        let buffers = vec![data.as_slice()];
        let nodes = load_gltf_nodes(gltf, &buffers, &vulkan_context, &set_layouts, buffer).unwrap();
        assert!(nodes.len() != 0);
    }

    #[test]
    pub fn test_hand() {
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();

        let (document, buffers, _) = gltf::import("../test_assets/hand.gltf").unwrap();
        let gltf = document.into_json().to_vec().unwrap();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();
        let buffer = vk::Buffer::null();
        let nodes =
            load_gltf_nodes(&gltf, &buffers, &vulkan_context, &set_layouts, buffer).unwrap();
        assert!(nodes.len() == 1);

        let hand = nodes.get("Hand").unwrap();

        let children = &hand.borrow().children;
        assert!(children.len() == 2);

        let palm = children.get(1).unwrap();
        assert!(palm.borrow().children.len() == 5);
        assert!(Rc::ptr_eq(&palm.borrow().parent.upgrade().unwrap(), hand));

        let hand_base = children.get(0).unwrap().borrow();
        assert!(Rc::ptr_eq(&hand_base.parent.upgrade().unwrap(), hand));

        let skin = hand_base.skin.as_ref().unwrap();
        assert_eq!(skin.inverse_bind_matrices.len(), 16);
        assert_eq!(skin.joints.len(), 16);

        let mesh = hand_base.mesh.as_ref().unwrap();
        let vertex_buffer = &mesh.vertex_buffer;

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
            let vertices: &[Vertex] =
                std::slice::from_raw_parts_mut(std::mem::transmute(memory), 2376);
            assert_eq!(vertices.len(), 2376);
            let first = &vertices[0];
            assert_eq!(
                first.joint_weights,
                vec4(0.67059284, 0.19407976, 0.115477115, 0.019850286)
            );
        }
    }

    #[test]
    pub fn test_simple() {
        let (document, buffers, _) = gltf::import("../test_assets/animation_test.gltf").unwrap();
        let buffers = buffers.iter().map(|b| b.0.as_slice()).collect();
        let vulkan_context = VulkanContext::testing().unwrap();
        let set_layouts = create_descriptor_set_layouts(&vulkan_context).unwrap();
        let ubo_buffer = vk::Buffer::null();

        let gltf_bytes = document.into_json().to_vec().unwrap();
        let nodes = load_gltf_nodes(
            &gltf_bytes,
            &buffers,
            &vulkan_context,
            &set_layouts,
            ubo_buffer,
        )
        .unwrap();

        let test = nodes.get("Test").unwrap();
        {
            let mut hand = test.borrow_mut();
            hand.active_animation_index.replace(0);
        }

        let test = test.borrow();
        assert_eq!(test.translation, vec3(0.0, 0.0, 0.0));
        assert_eq!(test.scale, vec3(1.0, 1.0, 1.0));
        assert_eq!(test.rotation, Quaternion::new(1.0, 0.0, 0.0, 0.0));
        let expected_matrix = Matrix4::from_scale(1.0);
        let node_matrix = test.get_node_matrix();
        assert_eq!(node_matrix, expected_matrix);
    }
}
