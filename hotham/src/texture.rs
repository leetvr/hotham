use anyhow::Result;
use ash::vk::{self};

use crate::{image::Image, resources::VulkanContext};

#[derive(Debug, Clone)]
pub struct Texture {
    pub image: Image,
    pub sampler: vk::Sampler,
    pub descriptor: vk::DescriptorImageInfo,
}

impl Texture {
    pub fn new(
        vulkan_context: &VulkanContext,
        image_buf: &Vec<u8>,
        width: u32,
        height: u32,
    ) -> Result<Self> {
        let (image, sampler) = vulkan_context.create_texture_image(image_buf, width, height)?;
        let descriptor = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(image.view)
            .sampler(sampler)
            .build();

        Ok(Texture {
            image,
            sampler,
            descriptor,
        })
    }
}
