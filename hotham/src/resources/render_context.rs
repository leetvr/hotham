use std::{ffi::CStr, mem::size_of, slice::from_ref as slice_from_ref};

pub static CLEAR_VALUES: [vk::ClearValue; 2] = [
    vk::ClearValue {
        color: vk::ClearColorValue {
            float32: [0.0, 0.0, 0.0, 1.0],
        },
    },
    vk::ClearValue {
        depth_stencil: vk::ClearDepthStencilValue {
            depth: 1.0,
            stencil: 0,
        },
    },
];

use crate::{
    rendering::{
        camera::Camera,
        descriptors::Descriptors,
        frame::Frame,
        image::Image,
        resources::Resources,
        scene_data::SceneData,
        swapchain::{Swapchain, SwapchainInfo},
        vertex::Vertex,
    },
    resources::{VulkanContext, XrContext},
    COLOR_FORMAT, DEPTH_FORMAT, VIEW_COUNT,
};
use anyhow::Result;
use ash::vk::{self, Handle};
use nalgebra::Matrix4;
use openxr as xr;
use vk_shader_macros::include_glsl;

static VERT: &[u32] = include_glsl!("src/shaders/pbr.vert", target: vulkan1_1);
static FRAG: &[u32] = include_glsl!("src/shaders/pbr.frag", target: vulkan1_1);
static COMPUTE: &[u32] = include_glsl!("src/shaders/culling.comp", target: vulkan1_1);
static SKYBOX_VERT: &[u32] = include_glsl!("src/shaders/skybox.vert", target: vulkan1_1);
static SKYBOX_FRAG: &[u32] = include_glsl!("src/shaders/skybox.frag", target: vulkan1_1);

// TODO: Is this a good idea?
pub const PIPELINE_DEPTH: usize = 2;

pub struct RenderContext {
    pub frames: [Frame; PIPELINE_DEPTH],
    pub frame_index: usize,
    pub pipeline: vk::Pipeline,
    pub compute_pipeline: vk::Pipeline,
    pub skybox_pipeline: vk::Pipeline,
    pub pipeline_layout: vk::PipelineLayout,
    pub compute_pipeline_layout: vk::PipelineLayout,
    pub skybox_pipeline_layout: vk::PipelineLayout,
    pub render_pass: vk::RenderPass,
    pub scene_data: SceneData,
    pub cameras: Vec<Camera>,
    pub views: Vec<xr::View>,
    pub resources: Resources,
    pub(crate) swapchain: Swapchain,
    pub(crate) descriptors: Descriptors,
}

impl RenderContext {
    pub fn new(vulkan_context: &VulkanContext, xr_context: &XrContext) -> Result<Self> {
        println!("[HOTHAM_RENDERER] Creating renderer..");
        let xr_swapchain = &xr_context.swapchain;
        let swapchain_resolution = xr_context.swapchain_resolution;

        // Build swapchain
        let swapchain = SwapchainInfo::from_openxr_swapchain(xr_swapchain, swapchain_resolution)?;
        Self::new_from_swapchain_info(vulkan_context, &swapchain)
    }

    pub(crate) fn new_from_swapchain_info(
        vulkan_context: &VulkanContext,
        swapchain_info: &SwapchainInfo,
    ) -> Result<Self> {
        let descriptors = unsafe { Descriptors::new(vulkan_context) };
        let resources = unsafe { Resources::new(vulkan_context, &descriptors) };

        // Pipeline, render pass
        let render_pass = create_render_pass(vulkan_context)?;
        let swapchain = Swapchain::new(swapchain_info, vulkan_context, render_pass);
        let pipeline_layout =
            create_pipeline_layout(vulkan_context, slice_from_ref(&descriptors.graphics_layout))?;
        let pipeline = create_pipeline(
            vulkan_context,
            pipeline_layout,
            &swapchain.render_area,
            render_pass,
        )?;
        let (compute_pipeline, compute_pipeline_layout) = create_compute_pipeline(
            &vulkan_context.device,
            slice_from_ref(&descriptors.compute_layout),
        );

        let (skybox_pipeline, skybox_pipeline_layout) =
            create_skybox_pipeline(vulkan_context, &swapchain.render_area, render_pass, &[]);

        // Create all the per-frame resources we need
        let mut index = 0;
        let frames = [(); PIPELINE_DEPTH].map(|_| {
            let frame =
                Frame::new(vulkan_context, index, &descriptors).expect("Unable to create frame!");
            // mmmm.. hacky
            index += 1;
            frame
        });

        let scene_data = Default::default();

        Ok(Self {
            frames,
            frame_index: 0,
            swapchain,
            pipeline,
            compute_pipeline,
            skybox_pipeline,
            pipeline_layout,
            compute_pipeline_layout,
            skybox_pipeline_layout,
            render_pass,
            cameras: vec![Default::default(); 2],
            views: vec![Default::default(); 2],
            scene_data,
            descriptors,
            resources,
        })
    }

    #[cfg(test)]
    pub(crate) fn testing() -> (Self, VulkanContext) {
        let vulkan_context = VulkanContext::testing().unwrap();
        let resolution = vk::Extent2D {
            height: 800,
            width: 800,
        };
        // Create an image with vulkan_context
        let image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::TRANSFER_SRC,
                2,
                1,
            )
            .unwrap();
        vulkan_context
            .set_debug_name(vk::ObjectType::IMAGE, image.handle.as_raw(), "Screenshot")
            .unwrap();

        let swapchain = SwapchainInfo {
            images: vec![image.handle],
            resolution,
        };

        (
            RenderContext::new_from_swapchain_info(&vulkan_context, &swapchain).unwrap(),
            vulkan_context,
        )
    }

    pub(crate) fn update_scene_data(&mut self, views: &[xr::View]) -> Result<()> {
        self.views = views.to_owned();

        // View (camera)
        let view_matrices = &self
            .cameras
            .iter_mut()
            .enumerate()
            .map(|(n, c)| c.update(&views[n]))
            .collect::<Result<Vec<_>>>()?;

        // Projection
        let near = 0.05;
        let far = 100.0;

        let fov_left = views[0].fov;
        let fov_right = views[1].fov;

        self.scene_data.view_projection = [
            get_projection(fov_left, near, far) * view_matrices[0],
            get_projection(fov_right, near, far) * view_matrices[1],
        ];

        self.scene_data.camera_position = [self.cameras[0].position(), self.cameras[1].position()];

        unsafe {
            let scene_data = &mut self.frames[self.frame_index]
                .scene_data_buffer
                .as_slice_mut()[0];
            scene_data.camera_position = self.scene_data.camera_position;
            scene_data.view_projection = self.scene_data.view_projection;
            scene_data.debug_data = self.scene_data.debug_data;
        }

        Ok(())
    }

    /// Start rendering a frame
    pub(crate) fn begin_frame(&self, vulkan_context: &VulkanContext) {
        // Get the values we need to start the frame..
        let device = &vulkan_context.device;
        let frame = &self.frames[self.frame_index];

        // Wait for the GPU to be ready.
        self.wait(device, frame);

        let command_buffer = frame.command_buffer;
        unsafe {
            device
                .begin_command_buffer(
                    command_buffer,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
        }
    }

    // TODO: GPU culling seems to cause deadlocks at high load. Most likely due to a clash between shared coherent memory.
    // Solution is to split buffers apart: https://github.com/leetvr/hotham/issues/263
    pub(crate) fn cull_objects(&mut self, vulkan_context: &VulkanContext) {
        let device = &vulkan_context.device;
        let frame_index = self.frame_index;
        let frame = &mut self.frames[self.frame_index];
        let draw_indirect_buffer = &frame.draw_indirect_buffer;
        let command_buffer = frame.command_buffer;
        let buffer_memory_barrier = vk::BufferMemoryBarrier::builder()
            .buffer(draw_indirect_buffer.buffer)
            .size(vk::WHOLE_SIZE)
            .src_access_mask(vk::AccessFlags::SHADER_WRITE)
            .dst_access_mask(vk::AccessFlags::INDIRECT_COMMAND_READ)
            .src_queue_family_index(0)
            .dst_queue_family_index(0);

        let near = 0.05;
        let far = 100.0;

        let fov_left = self.views[0].fov;
        let fov_right = self.views[1].fov;
        let fov = xr::Fovf {
            angle_left: fov_left.angle_left,
            angle_right: fov_right.angle_right,
            angle_up: fov_left.angle_up,
            angle_down: fov_left.angle_down,
        };

        // A virtual camera between the player's eyes
        let frustrum_projection = get_projection(fov, near, far);
        let frustrum_view = self.cameras[0]
            .position
            .lerp_slerp(&self.cameras[1].position, 0.5)
            .to_homogeneous()
            .try_inverse()
            .unwrap();

        let cull_data = CullData {
            frustrum: frustrum_projection * frustrum_view,
            draw_calls: draw_indirect_buffer.len as u32,
        };

        unsafe {
            frame.cull_data_buffer.overwrite(&[cull_data]);
        }

        let group_count_x = (draw_indirect_buffer.len / 256) + 1;

        unsafe {
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline_layout,
                0,
                slice_from_ref(&self.descriptors.compute_sets[frame_index]),
                &[],
            );
            device.cmd_dispatch(command_buffer, group_count_x as u32, 1, 1);
            device.cmd_pipeline_barrier(
                command_buffer,
                vk::PipelineStageFlags::COMPUTE_SHADER,
                vk::PipelineStageFlags::DRAW_INDIRECT,
                vk::DependencyFlags::empty(),
                &[],
                slice_from_ref(&buffer_memory_barrier),
                &[],
            );
        }
    }

    /// Begin the PBR renderpass.
    /// DOES NOT BEGIN RECORDING COMMAND BUFFERS - call begin_frame first!
    pub(crate) fn begin_pbr_render_pass(
        &self,
        vulkan_context: &VulkanContext,
        swapchain_image_index: usize,
    ) {
        // Get the values we need to start a renderpass
        let device = &vulkan_context.device;
        let frame = &self.frames[self.frame_index];
        let command_buffer = frame.command_buffer;
        let framebuffer = self.swapchain.framebuffers[swapchain_image_index];

        // Begin the renderpass.
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(self.swapchain.render_area)
            .clear_values(&CLEAR_VALUES);

        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &render_pass_begin_info,
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                slice_from_ref(&self.descriptors.sets[self.frame_index]),
                &[],
            );
            device.cmd_bind_index_buffer(
                command_buffer,
                self.resources.index_buffer.buffer,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_bind_vertex_buffers(
                command_buffer,
                0,
                slice_from_ref(&self.resources.vertex_buffer.buffer),
                &[0],
            );
        }
    }

    pub(crate) fn end_pbr_render_pass(&mut self, vulkan_context: &VulkanContext) {
        let device = &vulkan_context.device;
        let frame = &self.frames[self.frame_index];
        let command_buffer = frame.command_buffer;
        unsafe {
            device.cmd_end_render_pass(command_buffer);
        }
    }

    /// Finish rendering a frame
    pub(crate) fn end_frame(&mut self, vulkan_context: &VulkanContext) {
        // Get the values we need to end the renderpass
        let device = &vulkan_context.device;
        let frame = &self.frames[self.frame_index];
        let command_buffer = frame.command_buffer;
        let graphics_queue = vulkan_context.graphics_queue;

        // End the render pass and submit.
        unsafe {
            device.end_command_buffer(command_buffer).unwrap();
            let fence = frame.fence;
            let submit_info =
                vk::SubmitInfo::builder().command_buffers(slice_from_ref(&command_buffer));
            device
                .queue_submit(graphics_queue, slice_from_ref(&submit_info), fence)
                .expect("[HOTHAM_RENDER] @@ GPU CRASH DETECTED @@ - You are probably doing too much work in a compute shader!");
        }

        // And we're done! Bump the frame index.
        self.frame_index = (self.frame_index + 1) % PIPELINE_DEPTH;
    }

    pub(crate) fn wait(&self, device: &ash::Device, frame: &Frame) {
        let fence = frame.fence;

        unsafe {
            device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
            device.reset_fences(&[fence]).unwrap();
        }
    }

    pub(crate) fn create_texture_image(
        &mut self,
        name: &str,
        vulkan_context: &VulkanContext,
        image_buf: &[u8],
        mip_count: u32,
        offsets: Vec<vk::DeviceSize>,
        texture_image: &Image,
    ) -> Result<u32> {
        vulkan_context.set_debug_name(
            vk::ObjectType::IMAGE,
            texture_image.handle.as_raw(),
            name,
        )?;

        // TODO: This is only neccesary on desktop, or if there is data in the buffer!
        if !image_buf.is_empty() {
            vulkan_context.upload_image(image_buf, mip_count, offsets, texture_image);
        }

        let texture_index = unsafe {
            self.resources
                .write_texture_to_array(vulkan_context, &self.descriptors, texture_image)
        };

        println!(
            "[HOTHAM_VULKAN] ..done! Texture {} created successfully.",
            name
        );

        Ok(texture_index)
    }
}

pub fn create_push_constant<T: Sized>(p: &T) -> &[u8] {
    unsafe { std::slice::from_raw_parts(std::mem::transmute(p), size_of::<T>()) }
}

fn get_projection(fov: xr::Fovf, near: f32, far: f32) -> Matrix4<f32> {
    let tan_left = f32::tan(fov.angle_left);
    let tan_right = f32::tan(fov.angle_right);

    let tan_down = f32::tan(fov.angle_down);
    let tan_up = f32::tan(fov.angle_up);
    let tan_angle_width = tan_right - tan_left;
    let tan_angle_height = tan_down - tan_up;

    let c0r0 = 2.0 / tan_angle_width;
    let c1r0 = 0.0;
    let c2r0 = (tan_right + tan_left) / tan_angle_width;
    let c3r0 = 0.0;

    let c0r1 = 0.0;
    let c1r1 = 2.0 / tan_angle_height;
    let c2r1 = (tan_up + tan_down) / tan_angle_height;
    let c3r1 = 0.0;

    let c0r2 = 0.0;
    let c1r2 = 0.0;
    let c2r2 = -(far) / (far - near);
    let c3r2 = -(far * near) / (far - near);

    let c0r3 = 0.0;
    let c1r3 = 0.0;
    let c2r3 = -1.0;
    let c3r3 = 0.0;

    Matrix4::from_column_slice(&[
        c0r0, c0r1, c0r2, c0r3, c1r0, c1r1, c1r2, c1r3, c2r0, c2r1, c2r2, c2r3, c3r0, c3r1, c3r2,
        c3r3,
    ])
}

fn create_render_pass(vulkan_context: &VulkanContext) -> Result<vk::RenderPass> {
    print!("[HOTHAM_INIT] Creating render pass..");
    // Attachment used for MSAA
    let color_attachment = vk::AttachmentDescription::builder()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_4)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    // Final attachment to be presented
    let color_attachment_resolve = vk::AttachmentDescription::builder()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    let depth_attachment = vk::AttachmentDescription::builder()
        .format(DEPTH_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_4)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::DONT_CARE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let color_attachment_reference = vk::AttachmentReference::builder()
        .attachment(0)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let color_attachment_reference = [color_attachment_reference];

    let depth_stencil_reference = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL)
        .build();

    let color_attachment_resolve_reference = vk::AttachmentReference::builder()
        .attachment(2)
        .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
        .build();

    let color_attachment_resolve_reference = [color_attachment_resolve_reference];

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachment_reference)
        .resolve_attachments(&color_attachment_resolve_reference)
        .depth_stencil_attachment(&depth_stencil_reference);

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
        );

    let view_masks = [!(!0 << VIEW_COUNT)];
    let mut multiview = vk::RenderPassMultiviewCreateInfo::builder()
        .view_masks(&view_masks)
        .correlation_masks(&view_masks);

    let attachments = [
        *color_attachment,
        *depth_attachment,
        *color_attachment_resolve,
    ];

    let render_pass = unsafe {
        vulkan_context.device.create_render_pass(
            &vk::RenderPassCreateInfo::builder()
                .attachments(&attachments)
                .subpasses(&[*subpass])
                .dependencies(&[*dependency])
                .push_next(&mut multiview),
            None,
        )
    }?;
    println!("..done!");

    Ok(render_pass)
}

fn create_pipeline(
    vulkan_context: &VulkanContext,
    pipeline_layout: vk::PipelineLayout,
    render_area: &vk::Rect2D,
    render_pass: vk::RenderPass,
) -> Result<vk::Pipeline> {
    // Vertex shader stage
    let (vertex_shader, vertex_stage) =
        create_shader(VERT, vk::ShaderStageFlags::VERTEX, vulkan_context)?;

    // Fragment shader stage
    let (fragment_shader, fragment_stage) =
        create_shader(FRAG, vk::ShaderStageFlags::FRAGMENT, vulkan_context)?;

    let stages = [vertex_stage, fragment_stage];

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
        width: render_area.extent.width as _,
        height: render_area.extent.height as _,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let viewports = [viewport];

    // Scissors
    let scissors = [*render_area];

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    // Rasterization state
    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::BACK)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .rasterizer_discard_enable(false)
        .depth_clamp_enable(false)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0)
        .depth_bias_clamp(0.0)
        .depth_bias_slope_factor(0.0)
        .line_width(1.0);

    // Multisample state
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_4);

    // Depth stencil state
    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(true)
        .depth_compare_op(vk::CompareOp::LESS)
        .depth_bounds_test_enable(false)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)
        .stencil_test_enable(false);

    // Color blend state
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)
        .build();

    let color_blend_attachments = [color_blend_attachment];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();

    let create_infos = [create_info];

    let pipelines = unsafe {
        vulkan_context.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &create_infos,
            None,
        )
    }
    .map_err(|(_, r)| r)?;

    unsafe {
        vulkan_context
            .device
            .destroy_shader_module(vertex_shader, None);
        vulkan_context
            .device
            .destroy_shader_module(fragment_shader, None);
    }

    let primary_pipeline = pipelines[0];

    Ok(primary_pipeline)
}

pub fn create_shader(
    shader_code: &[u32],
    stage: vk::ShaderStageFlags,
    vulkan_context: &VulkanContext,
) -> Result<(vk::ShaderModule, vk::PipelineShaderStageCreateInfo)> {
    let main = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
    let create_info = vk::ShaderModuleCreateInfo::builder().code(shader_code);
    let shader_module = unsafe {
        vulkan_context
            .device
            .create_shader_module(&create_info, None)
    }?;
    let shader_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(stage)
        .name(<&std::ffi::CStr>::clone(&main))
        .module(shader_module)
        .build();

    Ok((shader_module, shader_stage))
}

fn create_pipeline_layout(
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
) -> Result<vk::PipelineLayout> {
    let create_info = &vk::PipelineLayoutCreateInfo::builder().set_layouts(set_layouts);
    unsafe {
        vulkan_context
            .device
            .create_pipeline_layout(create_info, None)
    }
    .map_err(|e| e.into())
}

#[repr(C)]
#[derive(Debug, Clone)]
pub struct CullData {
    /// View-Projection matrices (one per eye)
    pub frustrum: Matrix4<f32>,
    pub draw_calls: u32,
}

fn create_compute_pipeline(
    device: &ash::Device,
    layouts: &[vk::DescriptorSetLayout],
) -> (vk::Pipeline, vk::PipelineLayout) {
    unsafe {
        let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
        let compute_module = device
            .create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(COMPUTE), None)
            .unwrap();

        let create_info = &vk::PipelineLayoutCreateInfo::builder().set_layouts(layouts);

        let layout = device.create_pipeline_layout(create_info, None).unwrap();

        let create_info = vk::ComputePipelineCreateInfo::builder()
            .stage(vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::COMPUTE,
                module: compute_module,
                p_name: shader_entry_name.as_ptr(),
                ..Default::default()
            })
            .layout(layout);

        let pipeline = device
            .create_compute_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&create_info),
                None,
            )
            .unwrap()[0];

        (pipeline, layout)
    }
}

fn create_skybox_pipeline(
    vulkan_context: &VulkanContext,
    render_area: &vk::Rect2D,
    render_pass: vk::RenderPass,
    set_layouts: &[vk::DescriptorSetLayout],
) -> (vk::Pipeline, vk::PipelineLayout) {
    let pipeline_layout = unsafe {
        vulkan_context
            .device
            .create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(set_layouts)
                    .push_constant_ranges(slice_from_ref(
                        &vk::PushConstantRange::builder().size(128).stage_flags(vk::ShaderStageFlags::FRAGMENT),
                    )),
                None,
            )
            .unwrap()
    };
    // Vertex shader stage
    let (vertex_shader, vertex_stage) =
        create_shader(SKYBOX_VERT, vk::ShaderStageFlags::VERTEX, vulkan_context).unwrap();

    // Fragment shader stage
    let (fragment_shader, fragment_stage) =
        create_shader(SKYBOX_FRAG, vk::ShaderStageFlags::FRAGMENT, vulkan_context).unwrap();

    let stages = [vertex_stage, fragment_stage];
    let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder();

    // Input assembly state
    let input_assembly_state = vk::PipelineInputAssemblyStateCreateInfo::builder()
        .topology(vk::PrimitiveTopology::TRIANGLE_LIST);

    // Viewport State
    let viewport = vk::Viewport {
        x: 0.0,
        y: 0.0,
        width: render_area.extent.width as _,
        height: render_area.extent.height as _,
        min_depth: 0.0,
        max_depth: 1.0,
    };
    let viewports = [viewport];

    // Scissors
    let scissors = [*render_area];

    let viewport_state = vk::PipelineViewportStateCreateInfo::builder()
        .viewports(&viewports)
        .scissors(&scissors);

    // Rasterization state
    let rasterization_state = vk::PipelineRasterizationStateCreateInfo::builder()
        .polygon_mode(vk::PolygonMode::FILL)
        .cull_mode(vk::CullModeFlags::NONE)
        .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
        .rasterizer_discard_enable(false)
        .depth_clamp_enable(false)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0)
        .depth_bias_clamp(0.0)
        .depth_bias_slope_factor(0.0)
        .line_width(1.0);

    // Multisample state
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_4);

    // Depth stencil state
    let depth_stencil_state = vk::PipelineDepthStencilStateCreateInfo::builder()
        .depth_test_enable(true)
        .depth_write_enable(false)
        .depth_compare_op(vk::CompareOp::LESS_OR_EQUAL)
        .depth_bounds_test_enable(false)
        .min_depth_bounds(0.0)
        .max_depth_bounds(1.0)
        .stencil_test_enable(false);

    // Color blend state
    let color_blend_attachment = vk::PipelineColorBlendAttachmentState::builder()
        .color_write_mask(
            vk::ColorComponentFlags::R
                | vk::ColorComponentFlags::G
                | vk::ColorComponentFlags::B
                | vk::ColorComponentFlags::A,
        )
        .blend_enable(false)
        .build();

    let color_blend_attachments = [color_blend_attachment];

    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op_enable(false)
        .attachments(&color_blend_attachments);

    let create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&stages)
        .vertex_input_state(&vertex_input_state)
        .input_assembly_state(&input_assembly_state)
        .viewport_state(&viewport_state)
        .rasterization_state(&rasterization_state)
        .multisample_state(&multisample_state)
        .depth_stencil_state(&depth_stencil_state)
        .color_blend_state(&color_blend_state)
        .layout(pipeline_layout)
        .render_pass(render_pass)
        .subpass(0)
        .build();

    let create_infos = [create_info];

    let pipelines = unsafe {
        vulkan_context.device.create_graphics_pipelines(
            vk::PipelineCache::null(),
            &create_infos,
            None,
        )
    }
    .unwrap();

    unsafe {
        vulkan_context
            .device
            .destroy_shader_module(vertex_shader, None);
        vulkan_context
            .device
            .destroy_shader_module(fragment_shader, None);
    }

    let primary_pipeline = pipelines[0];

    (primary_pipeline, pipeline_layout)
}
