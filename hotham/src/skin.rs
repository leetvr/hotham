use std::{cell::RefCell, mem::size_of, rc::Rc};

use crate::{buffer::Buffer, node::Node, resources::VulkanContext};
use anyhow::{anyhow, Result};
use ash::vk;
use cgmath::Matrix4;

type Joints = Vec<Rc<RefCell<Node>>>;

#[derive(Debug, Clone)]
pub struct Skin {
    pub skeleton_root_index: usize,
    pub name: String,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
    pub joints: Joints,
    pub descriptor_sets: Vec<vk::DescriptorSet>,
    pub(crate) ssbo: Buffer<Matrix4<f32>>,
}

impl Skin {
    pub(crate) fn load(
        skin_data: &gltf::Skin,
        buffers: &Vec<&[u8]>,
        vulkan_context: &VulkanContext,
        parent_node: Rc<RefCell<Node>>,
        skin_descriptor_set_layout: vk::DescriptorSetLayout,
    ) -> Result<()> {
        let name = skin_data.name().unwrap_or("Skin").to_string();
        let inverse_bind_matrices = skin_data
            .reader(|buffer| Some(&buffers[buffer.index()]))
            .read_inverse_bind_matrices()
            .ok_or_else(|| anyhow!("No inverse bind matrices"))?
            .map(|matrix| matrix.into())
            .collect::<Vec<_>>();

        let joints = load_joints(skin_data, &parent_node)?;
        let ssbo = Buffer::new_from_vec(
            vulkan_context,
            &inverse_bind_matrices,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;
        let size = size_of::<Matrix4<f32>>() * inverse_bind_matrices.len();
        let descriptor_sets = vulkan_context.create_skin_descriptor_set(
            skin_descriptor_set_layout,
            ssbo.handle,
            size,
        )?;

        let skeleton_root_index = skin_data.joints().next().unwrap().index();

        let skin = Skin {
            inverse_bind_matrices,
            skeleton_root_index,
            name,
            joints,
            ssbo,
            descriptor_sets,
        };

        parent_node.borrow_mut().skin.replace(skin);
        Ok(())
    }
}

fn load_joints(skin_data: &gltf::Skin, skeleton_root: &Rc<RefCell<Node>>) -> Result<Joints> {
    skin_data
        .joints()
        .map(|joint| {
            let index = joint.index();
            (**skeleton_root).borrow().find(index).ok_or_else(|| {
                anyhow!(
                    "Unable to find node with index {} in node {:?}",
                    index,
                    skeleton_root
                )
            })
        })
        .collect::<Result<Joints>>()
}
