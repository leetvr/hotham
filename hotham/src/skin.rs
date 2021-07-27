use std::{cell::RefCell, rc::Rc};

use crate::{buffer::Buffer, node::Node, vulkan_context::VulkanContext};
use anyhow::{anyhow, Result};
use ash::vk;
use cgmath::Matrix4;

type Joints = Vec<Rc<RefCell<Node>>>;

#[derive(Debug, Clone)]
pub struct Skin {
    pub skeleton_root: Rc<RefCell<Node>>,
    pub name: String,
    pub inverse_bind_matrices: Vec<Matrix4<f32>>,
    pub joints: Joints,
    pub(crate) ssbo: Buffer<Matrix4<f32>>,
}

impl Skin {
    pub(crate) fn load(
        skin_data: &gltf::Skin,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        skeleton_root: Rc<RefCell<Node>>,
    ) -> Result<()> {
        let name = skin_data.name().unwrap_or("Skin").to_string();
        let inverse_bind_matrices = skin_data
            .reader(|buffer| Some(&blob[buffer.index()..buffer.index() + buffer.length()]))
            .read_inverse_bind_matrices()
            .ok_or_else(|| anyhow!("No inverse bind matrices"))?
            .map(|matrix| matrix.into())
            .collect::<Vec<_>>();

        let joints = load_joints(skin_data, &skeleton_root)?;
        let ssbo = Buffer::new_from_vec(
            vulkan_context,
            &inverse_bind_matrices,
            vk::BufferUsageFlags::STORAGE_BUFFER,
        )?;

        let skin = Skin {
            inverse_bind_matrices,
            skeleton_root: skeleton_root.clone(),
            name,
            joints,
            ssbo,
        };

        skeleton_root.borrow_mut().skin.replace(skin);
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
