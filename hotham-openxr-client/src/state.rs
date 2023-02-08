use lazy_vulkan::vulkan_context::VulkanContext;
use openxr_sys::Instance;

pub struct State {
    instance: Instance,
    vulkan_context: Option<VulkanContext>,
}

impl State {
    pub fn new(instance: Instance) -> Self {
        Self {
            instance,
            vulkan_context: None,
        }
    }
}
