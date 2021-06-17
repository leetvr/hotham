use crate::Result;
use ash::vk::{self, Handle};
use openxr::{Swapchain as SwapchainHandle, Vulkan, VulkanLegacy};

pub struct Swapchain {
    pub resolution: vk::Extent2D,
    pub images: Vec<vk::Image>,
}

impl Swapchain {
    pub(crate) fn new(
        handle: &SwapchainHandle<VulkanLegacy>,
        resolution: vk::Extent2D,
    ) -> Result<Self> {
        print!("[HOTHAM_INIT] Creating swapchain..");

        let images = handle
            .enumerate_images()?
            .into_iter()
            .map(vk::Image::from_raw)
            .collect::<Vec<_>>();

        println!("..done!");

        Ok(Self { resolution, images })
    }
}
