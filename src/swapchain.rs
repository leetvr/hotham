use openxr::{Session, Swapchain as SwapchainHandle, Vulkan};

use crate::{vulkan_context::VulkanContext, Result};

pub struct Swapchain {
    handle: SwapchainHandle<Vulkan>,
}

impl Swapchain {
    pub(crate) fn new(context: &VulkanContext, session: &Session<Vulkan>) -> Result<Self> {
        let handle = todo!();
        Ok(Self { handle })
    }
}
