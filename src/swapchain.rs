use ash::vk::{self, Handle};
use openxr::{
    Instance, Session, Swapchain as SwapchainHandle, SwapchainCreateFlags, SwapchainCreateInfo,
    SwapchainUsageFlags, SystemId, Vulkan,
};

use crate::{Result, COLOR_FORMAT, VIEW_COUNT, VIEW_TYPE};

pub struct Swapchain {
    pub handle: SwapchainHandle<Vulkan>,
    pub resolution: vk::Extent2D,
    pub images: Vec<vk::Image>,
}

impl Swapchain {
    pub(crate) fn new(
        xr_session: &Session<Vulkan>,
        xr_instance: &Instance,
        system: SystemId,
    ) -> Result<Self> {
        let views = xr_instance.enumerate_view_configuration_views(system, VIEW_TYPE)?;
        let resolution = vk::Extent2D {
            width: views[0].recommended_image_rect_width,
            height: views[0].recommended_image_rect_height,
        };

        let handle = xr_session.create_swapchain(&SwapchainCreateInfo {
            create_flags: SwapchainCreateFlags::EMPTY,
            usage_flags: SwapchainUsageFlags::COLOR_ATTACHMENT,
            format: COLOR_FORMAT.as_raw() as u32,
            sample_count: 1,
            width: resolution.width,
            height: resolution.height,
            face_count: 1,
            array_size: VIEW_COUNT,
            mip_count: 1,
        })?;

        let images = handle
            .enumerate_images()?
            .into_iter()
            .map(vk::Image::from_raw)
            .collect::<Vec<_>>();

        Ok(Self {
            handle,
            resolution,
            images,
        })
    }
}
