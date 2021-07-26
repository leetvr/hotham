use std::{cell::RefCell, rc::Rc};

use crate::{node::Node, vulkan_context::VulkanContext};
use anyhow::Result;
use ash::vk;

#[derive(Debug, Clone)]
pub struct Skin {
    pub skeleton_root: Rc<RefCell<Node>>,
    pub name: String,
    // std::string            name;
    // Node *                 skeletonRoot = nullptr;
    // std::vector<glm::mat4> inverseBindMatrices;
    // std::vector<Node *>    joints;
    // vks::Buffer            ssbo;
    // VkDescriptorSet        descriptorSet;
}

impl Skin {
    pub(crate) fn load(
        skin_data: &gltf::Skin,
        blob: &[u8],
        vulkan_context: &VulkanContext,
        set_layouts: &[vk::DescriptorSetLayout],
        skeleton_root: Rc<RefCell<Node>>,
    ) -> Result<Self> {
        let name = skin_data.name().unwrap_or("Skin").to_string();

        Ok(Skin {
            skeleton_root,
            name,
        })
    }
}
