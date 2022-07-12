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
        camera::Camera, descriptors::Descriptors, frame::Frame, image::Image, resources::Resources,
        scene_data::SceneData, swapchain::Swapchain, texture::DEFAULT_COMPONENT_MAPPING,
        vertex::Vertex,
    },
    resources::{VulkanContext, XrContext},
    COLOR_FORMAT, DEPTH_ATTACHMENT_USAGE_FLAGS, DEPTH_FORMAT, VIEW_COUNT,
};
use anyhow::Result;
use ash::vk::{self, Handle};
use nalgebra::Matrix4;
use openxr as xr;
use vk_shader_macros::include_glsl;

static VERT: &[u32] = include_glsl!("src/shaders/pbr.vert");
static FRAG: &[u32] = include_glsl!("src/shaders/pbr.frag");
static COMPUTE: &[u32] = include_glsl!("src/shaders/culling.comp");

pub struct RenderContext {
    pub frames: Vec<Frame>,
    pub pipeline_layout: vk::PipelineLayout,
    pub pipeline: vk::Pipeline,
    pub compute_pipeline: vk::Pipeline,
    pub render_pass: vk::RenderPass,
    pub depth_image: Image,
    pub color_image: Image,
    pub render_area: vk::Rect2D,
    pub scene_data: SceneData,
    pub cameras: Vec<Camera>,
    pub views: Vec<xr::View>,
    pub frame_index: usize,
    pub resources: Resources,
    pub(crate) descriptors: Descriptors,
}

impl RenderContext {
    pub fn new(vulkan_context: &VulkanContext, xr_context: &XrContext) -> Result<Self> {
        println!("[HOTHAM_RENDERER] Creating renderer..");
        let xr_swapchain = &xr_context.swapchain;
        let swapchain_resolution = xr_context.swapchain_resolution;

        // Build swapchain
        let swapchain = Swapchain::new(xr_swapchain, swapchain_resolution)?;
        Self::new_from_swapchain(vulkan_context, &swapchain)
    }

    pub(crate) fn new_from_swapchain(
        vulkan_context: &VulkanContext,
        swapchain: &Swapchain,
    ) -> Result<Self> {
        let render_area = vk::Rect2D {
            extent: swapchain.resolution,
            offset: vk::Offset2D::default(),
        };

        let descriptors = unsafe { Descriptors::new(vulkan_context) };
        let mut resources = unsafe { Resources::new(vulkan_context, &descriptors) };

        // Pipeline, render pass
        let render_pass = create_render_pass(vulkan_context)?;
        let pipeline_layout =
            create_pipeline_layout(vulkan_context, slice_from_ref(&descriptors.layout))?;
        let pipeline = create_pipeline(vulkan_context, pipeline_layout, &render_area, render_pass)?;
        let compute_pipeline = create_compute_pipeline(&vulkan_context.device, pipeline_layout);

        // Depth image, shared between frames
        let depth_image = vulkan_context.create_image(
            DEPTH_FORMAT,
            &swapchain.resolution,
            DEPTH_ATTACHMENT_USAGE_FLAGS,
            2,
            1,
        )?;

        // Color image, used for MSAA.
        let color_image = vulkan_context.create_image(
            COLOR_FORMAT,
            &swapchain.resolution,
            vk::ImageUsageFlags::TRANSIENT_ATTACHMENT | vk::ImageUsageFlags::COLOR_ATTACHMENT,
            2,
            1,
        )?;

        // Create all the per-frame resources we need
        let frames = create_frames(
            vulkan_context,
            &render_pass,
            swapchain,
            &depth_image,
            &color_image,
        )?;
        let scene_data = Default::default();
        unsafe {
            resources.scene_data_buffer.push(&scene_data);
        }

        Ok(Self {
            frames,
            pipeline,
            compute_pipeline,
            pipeline_layout,
            render_pass,
            frame_index: 0,
            depth_image,
            color_image,
            render_area,
            cameras: vec![Default::default(); 2],
            views: Vec::new(),
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

        let swapchain = Swapchain {
            images: vec![image.handle],
            resolution,
        };

        (
            RenderContext::new_from_swapchain(&vulkan_context, &swapchain).unwrap(),
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

        let view_projection = [
            get_projection(fov_left, near, far) * view_matrices[0],
            get_projection(fov_right, near, far) * view_matrices[1],
        ];

        let camera_position = [self.cameras[0].position(), self.cameras[1].position()];

        unsafe {
            let scene_data = &mut self.resources.scene_data_buffer.as_slice_mut()[0];
            scene_data.camera_position = camera_position;
            scene_data.view_projection = view_projection;
            scene_data.debug_data = self.scene_data.debug_data;
        }

        Ok(())
    }

    pub(crate) fn begin_frame(&self, vulkan_context: &VulkanContext, swapchain_image_index: usize) {
        // Get the values we need to start the frame..
        let device = &vulkan_context.device;
        let frame = &self.frames[swapchain_image_index];

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

    // TODO: Get culling working correctly. Requires separate queue, fence and command buffer.
    // https://github.com/leetvr/hotham/issues/226
    pub(crate) fn _cull_objects(
        &self,
        vulkan_context: &VulkanContext,
        swapchain_image_index: usize,
    ) {
        let device = &vulkan_context.device;
        let frame = &self.frames[swapchain_image_index];
        let command_buffer = frame.command_buffer;
        let fence = frame.fence;

        unsafe {
            device
                .begin_command_buffer(
                    command_buffer,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.compute_pipeline,
            );
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::COMPUTE,
                self.pipeline_layout,
                0,
                slice_from_ref(&self.descriptors.set),
                &[],
            );
            device.cmd_dispatch(
                command_buffer,
                self.resources.draw_indirect_buffer.len as u32,
                1,
                1,
            );
            device.end_command_buffer(command_buffer).unwrap();

            let submit_info =
                vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&command_buffer));
            device
                .queue_submit(
                    vulkan_context.graphics_queue, // TODO: get a proper queue
                    std::slice::from_ref(&submit_info),
                    fence,
                )
                .unwrap();
            device.wait_for_fences(&[fence], true, 1000000000).unwrap();
            device.reset_fences(&[fence]).unwrap();
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
        let frame = &self.frames[swapchain_image_index];
        let command_buffer = frame.command_buffer;
        let framebuffer = frame.framebuffer;

        // Begin the renderpass.
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(self.render_area)
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
                slice_from_ref(&self.descriptors.set),
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

    pub(crate) fn end_pbr_render_pass(
        &mut self,
        vulkan_context: &VulkanContext,
        swapchain_image_index: usize,
    ) {
        let device = &vulkan_context.device;
        let frame = &self.frames[swapchain_image_index];
        let command_buffer = frame.command_buffer;
        unsafe {
            device.cmd_end_render_pass(command_buffer);
        }
    }

    pub(crate) fn end_frame(
        &mut self,
        vulkan_context: &VulkanContext,
        swapchain_image_index: usize,
    ) {
        // Get the values we need to end the renderpass
        let device = &vulkan_context.device;
        let frame = &self.frames[swapchain_image_index];
        let command_buffer = frame.command_buffer;
        let graphics_queue = vulkan_context.graphics_queue;

        // End the render pass and submit.
        unsafe {
            device.end_command_buffer(command_buffer).unwrap();
            let fence = frame.fence;
            let submit_info = vk::SubmitInfo::builder()
                .command_buffers(&[command_buffer])
                .build();
            device
                .queue_submit(graphics_queue, &[submit_info], fence)
                .unwrap();
        }

        self.frame_index += 1;
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

fn create_frames(
    vulkan_context: &VulkanContext,
    render_pass: &vk::RenderPass,
    swapchain: &Swapchain,
    depth_image: &Image,
    color_image: &Image,
) -> Result<Vec<Frame>> {
    print!("[HOTHAM_INIT] Creating frames..");
    let frames = swapchain
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
        .map(|i| {
            Frame::new(
                vulkan_context,
                *render_pass,
                swapchain.resolution,
                i,
                depth_image.view,
                color_image.view,
            )
        })
        .collect::<Result<Vec<Frame>>>()?;
    println!(" ..done!");
    Ok(frames)
}

fn create_render_pass(vulkan_context: &VulkanContext) -> Result<vk::RenderPass> {
    print!("[HOTHAM_INIT] Creating render pass..");
    // Attachment used for MSAA
    let color_attachment = vk::AttachmentDescription::builder()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_4)
        .load_op(vk::AttachmentLoadOp::CLEAR)
        .store_op(vk::AttachmentStoreOp::STORE)
        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .final_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL);

    // Final attachment to be presented
    let color_attachment_resolve = vk::AttachmentDescription::builder()
        .format(COLOR_FORMAT)
        .samples(vk::SampleCountFlags::TYPE_1)
        .load_op(vk::AttachmentLoadOp::DONT_CARE)
        .store_op(vk::AttachmentStoreOp::STORE)
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
    print!("[HOTHAM_INIT] Creating pipeline..");
    // Build up the state of the pipeline

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

fn create_compute_pipeline(device: &ash::Device, layout: vk::PipelineLayout) -> vk::Pipeline {
    unsafe {
        let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
        let compute_module = device
            .create_shader_module(&vk::ShaderModuleCreateInfo::builder().code(COMPUTE), None)
            .unwrap();
        let create_info = vk::ComputePipelineCreateInfo::builder()
            .stage(vk::PipelineShaderStageCreateInfo {
                stage: vk::ShaderStageFlags::COMPUTE,
                module: compute_module,
                p_name: shader_entry_name.as_ptr(),
                ..Default::default()
            })
            .layout(layout);

        device
            .create_compute_pipelines(
                vk::PipelineCache::null(),
                std::slice::from_ref(&create_info),
                None,
            )
            .unwrap()[0]
    }
}
