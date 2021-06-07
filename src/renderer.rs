use std::mem::size_of;

use crate::{
    frame::Frame, hotham_error::HothamError, swapchain::Swapchain, vulkan_context::VulkanContext,
    ProgramInitialization, Result, Vertex, COLOR_FORMAT, DEPTH_FORMAT,
};
use anyhow::Context;
use ash::{version::DeviceV1_0, vk};
use openxr as xr;
use xr::Vulkan;

pub(crate) struct Renderer {
    context: VulkanContext,
    swapchain: Swapchain,
    frames: Vec<Frame>,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.context
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.context
                .device
                .destroy_render_pass(self.render_pass, None);
        }
    }
}

impl Renderer {
    pub(crate) fn new(
        vulkan_context: VulkanContext,
        xr_session: &xr::Session<Vulkan>,
        xr_instance: &xr::Instance,
        system: xr::SystemId,
        params: &ProgramInitialization,
    ) -> Result<Self> {
        println!("Creating renderer..");
        let swapchain = Swapchain::new(xr_session, xr_instance, system)?;
        let pipeline_layout = create_pipeline_layout(&vulkan_context)?;
        let render_pass = create_render_pass(&vulkan_context)?;
        let pipeline = create_pipeline(
            &vulkan_context,
            pipeline_layout,
            params,
            &swapchain.resolution,
            render_pass,
        )?;
        let frames = create_frames(&vulkan_context)?;

        Ok(Self {
            swapchain,
            context: vulkan_context,
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

fn create_frames(vulkan_context: &VulkanContext) -> Result<Vec<Frame>> {
    todo!()
}

fn create_render_pass(vulkan_context: &VulkanContext) -> Result<vk::RenderPass> {
    let color_attachment = vk::AttachmentDescription::builder()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let depth_attachment = vk::AttachmentDescription::builder()
        .format(DEPTH_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    let attachments = [color_attachment, depth_attachment];

    let color_attachment_reference = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();
    let color_attachments = [color_attachment_reference];

    let depth_stencil_attachment = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .depth_stencil_attachment(&depth_stencil_attachment)
        .build();
    let subpasses = [subpass];

    let dependency = vk::SubpassDependency::builder()
        .src_subpass(vk::SUBPASS_EXTERNAL)
        .dst_subpass(0)
        .src_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .src_access_mask(vk::AccessFlags::empty())
        .dst_stage_mask(
            vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                | vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
        )
        .dst_access_mask(
            vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                | vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
        )
        .build();
    let dependencies = [dependency];

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies);

    unsafe {
        vulkan_context
            .device
            .create_render_pass(&create_info, None)
            .map_err(Into::into)
    }
}

fn create_pipeline(
    vulkan_context: &VulkanContext,
    layout: vk::PipelineLayout,
    params: &ProgramInitialization,
    swapchain_resolution: &vk::Extent2D,
    render_pass: vk::RenderPass,
) -> Result<vk::Pipeline> {
    // Build up the state of the pipeline

    // Vertex shader stage
    let vertex_code = read_spv_from_path(params.vertex_shader)?;
    let vertex_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);
    let vertex_shader = unsafe {
        vulkan_context
            .device
            .create_shader_module(&vertex_shader_create_info, None)
    }?;
    let vertex_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .module(vertex_shader)
        .build();

    // Fragment shader stage
    let fragment_code = read_spv_from_path(params.fragment_shader)?;
    let fragment_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&fragment_code);
    let fragment_shader = unsafe {
        vulkan_context
            .device
            .create_shader_module(&fragment_shader_create_info, None)
    }?;
    let fragment_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .module(fragment_shader)
        .build();

    // Vertex input state
    let vertex_binding_description = vk::VertexInputBindingDescription::builder()
        .binding(0)
        .stride(size_of::<Vertex>() as _)
        .input_rate(vk::VertexInputRate::VERTEX)
        .build();
    let vertex_binding_descriptions = [vertex_binding_description];
    let vertex_attribute_descriptions = Vertex::attribute_descriptions();

    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(&vertex_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_binding_descriptions);

    // Input assembly state
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    // Viewport State
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: swapchain_resolution.width as _,
        height: swapchain_resolution.height as _,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let viewports = [viewport];

    // Scissors
    let mut scissor = vk::Rect2D::default();
    scissor.extent = *swapchain_resolution;
    let scissors = [scissor];

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    // Rasterization state
    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .depth_clamp_enable(false)
        .rasterizer_discard_enable(false)
        .depth_bias_enable(false)
        .line_width(1.0);

    // Multisample State
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .stencil_test_enable(false);

    let create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&[vertex_stage, fragment_stage])
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .layout(layout)
        .render_pass(render_pass)
        .build();

    let mut pipelines = unsafe {
        vulkan_context.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &[create_info],
            None,
        )
    }
    .map_err(|(_, r)| r)?;

    pipelines.pop().ok_or(HothamError::EmptyListError.into())
}

fn read_spv_from_path(path: &std::path::Path) -> Result<Vec<u32>> {
    let mut file = std::fs::File::open(path)
        .with_context(|| format!("Failed to read SPV file at {:?}", path))?;
    ash::util::read_spv(&mut file)
        .with_context(|| format!("Unable to read SPV file at {:?}", path))
        .map_err(|e| e.into())
}

fn create_pipeline_layout(vulkan_context: &VulkanContext) -> Result<vk::PipelineLayout> {
    unsafe {
        vulkan_context
            .device
            .create_pipeline_layout(&Default::default(), None)
    }
    .map_err(|e| e.into())
}
