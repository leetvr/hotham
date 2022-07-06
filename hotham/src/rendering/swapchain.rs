use anyhow::Result;
use ash::vk::{self, Handle};
use openxr::{Swapchain as SwapchainHandle, Vulkan};

/// A thin conatiner for the contents of the swapchain
pub struct Swapchain {
    /// The resolution of the swapchain
    pub resolution: vk::Extent2D,
    /// The images held in the swapchain
    pub images: Vec<vk::Image>,
}

impl Swapchain {
    pub(crate) fn new(handle: &SwapchainHandle<Vulkan>, resolution: vk::Extent2D) -> Result<Self> {
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
