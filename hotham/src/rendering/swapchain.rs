use anyhow::Result;
use ash::vk::{self, Handle};
use openxr::{Swapchain as SwapchainHandle, Vulkan};
use vulkan_context::VulkanContext;

use crate::{
    contexts::{render_context::USE_MSAA, vulkan_context},
    COLOR_FORMAT, DEPTH_FORMAT,
};

use super::texture::DEFAULT_COMPONENT_MAPPING;

/// A thin container for OpenXR to pass the details of its Swapchain to RenderContext.
pub struct SwapchainInfo {
    /// The resolution of the swapchain
    pub resolution: vk::Extent2D,
    /// The images held in the swapchain
    pub images: Vec<vk::Image>,
    /// Optional images used for fixed foveated rendering
    pub ffr_image: Option<FFRImage>,
}

impl SwapchainInfo {
    pub(crate) fn from_openxr_swapchain(
        handle: &SwapchainHandle<Vulkan>,
        resolution: vk::Extent2D,
    ) -> Result<Self> {
        // let images = handle
        //     .enumerate_images()?
        //     .into_iter()
        //     .map(vk::Image::from_raw)
        //     .collect::<Vec<_>>();

        let (images, ffr_image) = get_ffr_image(handle);

        Ok(Self {
            resolution,
            images,
            ffr_image,
        })
    }
}

/// Image for fixed foveated rendering
#[derive(Debug, Clone)]
pub struct FFRImage {
    /// Resolution of the image
    pub resolution: vk::Extent2D,
    /// Vulkan handle to the image
    pub image: vk::Image,
}

fn get_ffr_image(handle: &SwapchainHandle<Vulkan>) -> (Vec<vk::Image>, Option<FFRImage>) {
    let fp = handle.instance().fp();
    let mut output = 0;

    println!("[HOTHAM_SWAPCHAIN] Getting FFR image..");
    let images = unsafe {
        let result =
            (fp.enumerate_swapchain_images)(handle.as_raw(), 0, &mut output, std::ptr::null_mut());
        if result.into_raw() < 0 {
            panic!("Error getting swapchain images? {result:?}");
        }

        let mut swapchain_foveation_vulkan =
            openxr::sys::SwapchainImageFoveationVulkanFB::out(std::ptr::null_mut() as _);
        let mut buf = vec![
            openxr::sys::SwapchainImageVulkanKHR {
                ty: openxr::sys::SwapchainImageVulkanKHR::TYPE,
                next: &mut swapchain_foveation_vulkan as *mut _ as *mut std::ffi::c_void,
                image: 0,
            };
            output as _
        ];

        let result = (fp.enumerate_swapchain_images)(
            handle.as_raw(),
            output,
            &mut output,
            buf.as_mut_ptr() as _,
        );

        if result.into_raw() < 0 {
            panic!("Error getting swapchain images? {result:?}");
        }

        buf
    };

    println!("Got images: {images:?}");

    let ffr_image =
        unsafe { *(images[0].next as *mut openxr::sys::SwapchainImageFoveationVulkanFB) };
    let ffr_image = FFRImage {
        resolution: vk::Extent2D {
            width: ffr_image.width,
            height: ffr_image.height,
        },
        image: vk::Image::from_raw(ffr_image.image),
    };

    let images = images
        .iter()
        .map(|i| vk::Image::from_raw(i.image))
        .collect();

    (images, Some(ffr_image))
}

/// A thin container for OpenXR to pass the details of its Swapchain to RenderContext.
pub struct Swapchain {
    /// The dimensions of the swapchain.
    pub render_area: vk::Rect2D,
    /// The framebuffers of the swapchain, one per swapchain image.
    pub framebuffers: Vec<vk::Framebuffer>,
}

impl Swapchain {
    /// Create a swapchain
    pub fn new(
        swapchain_info: &SwapchainInfo,
        vulkan_context: &VulkanContext,
        render_pass: vk::RenderPass,
    ) -> Self {
        let render_area = vk::Rect2D {
            extent: swapchain_info.resolution,
            ..Default::default()
        };

        // Depth image, shared between frames
        let depth_image = vulkan_context
            .create_image(
                DEPTH_FORMAT,
                &swapchain_info.resolution,
                vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT
                    | vk::ImageUsageFlags::TRANSIENT_ATTACHMENT,
                2,
                1,
            )
            .unwrap();

        // Color image, used for MSAA.
        let color_image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &swapchain_info.resolution,
                vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
                2,
                1,
            )
            .unwrap();

        // Framebuffers, used for rendering the final image to the swapchain.
        let framebuffers = swapchain_info
            .images
            .iter()
            .flat_map(|i| {
                vulkan_context.create_image_view(
                    i,
                    COLOR_FORMAT,
                    vk::ImageViewType::TYPE_2D_ARRAY,
                    2,
                    1,
                    DEFAULT_COMPONENT_MAPPING,
                )
            })
            .map(|swapchain_image_view| {
                let msaa_attachments = [color_image.view, depth_image.view, swapchain_image_view];
                let non_msaa_attachments = [swapchain_image_view, depth_image.view];

                let frame_buffer_create_info = if USE_MSAA {
                    vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(&msaa_attachments)
                        .width(swapchain_info.resolution.width)
                        .height(swapchain_info.resolution.height)
                        .layers(1) // NOTE: multiview takes care of layers.
                } else {
                    vk::FramebufferCreateInfo::builder()
                        .render_pass(render_pass)
                        .attachments(&non_msaa_attachments)
                        .width(swapchain_info.resolution.width)
                        .height(swapchain_info.resolution.height)
                        .layers(1) // NOTE: multiview takes care of layers.
                };

                unsafe {
                    vulkan_context
                        .device
                        .create_framebuffer(&frame_buffer_create_info, None)
                }
                .unwrap()
            })
            .collect();

        Self {
            render_area,
            framebuffers,
        }
    }
}
