use crate::{frame::Frame, swapchain::Swapchain, vulkan_context::VulkanContext, Result, Vertex};
use ash::vk;
use openxr as xr;
use xr::Vulkan;

pub(crate) struct Renderer {
    context: VulkanContext,
    swapchain: Swapchain,
    frames: Vec<Frame>,
    pipeline: vk::Pipeline,
    pipeline_layout: vk::PipelineLayout,
    render_pass: vk::RenderPass,
}

impl Renderer {
    pub(crate) fn new(context: VulkanContext, xr_session: &xr::Session<Vulkan>) -> Result<Self> {
        let swapchain = Swapchain::new(&context, xr_session)?;
        let frames = Vec::new();
        let pipeline = todo!();
        let pipeline_layout = todo!();
        let render_pass = todo!();

        Ok(Self {
            swapchain,
            context,
            frames,
            pipeline,
            pipeline_layout,
            render_pass,
        })
    }

    pub fn update(&self, vertices: &Vec<Vertex>, indices: &Vec<u32>) -> () {
        println!("Vertices are now: {:?}", vertices);
        println!("Indices are now: {:?}", indices);
    }
}
