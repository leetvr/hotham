use std::{ffi::CStr, mem::size_of, time::Instant, u64};

use crate::{
    buffer::Buffer, camera::Camera, frame::Frame, hotham_error::HothamError, image::Image,
    swapchain::Swapchain, vulkan_context::VulkanContext, ProgramInitialization,
    UniformBufferObject, Vertex, COLOR_FORMAT, DEPTH_FORMAT, VIEW_COUNT,
};
use anyhow::Result;
use ash::{prelude::VkResult, version::DeviceV1_0, vk};
use cgmath::{vec3, Deg, Euler, Matrix4};
use console::Term;
use openxr as xr;

use xr::VulkanLegacy;

pub(crate) struct Renderer {
    _swapchain: Swapchain,
    vulkan_context: VulkanContext,
    frames: Vec<Frame>,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    depth_image: Image,
    render_area: vk::Rect2D,
    vertex_buffer: Buffer<Vertex>,
    index_buffer: Buffer<u32>,
    uniform_buffer: Buffer<UniformBufferObject>,
    uniform_buffer_descriptor_set: vk::DescriptorSet,
    render_start_time: Instant,
    cameras: Vec<Camera>,
    term: Term,
    views: Vec<xr::View>,
    pub frame_index: usize,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_context
                .device
                .queue_wait_idle(self.vulkan_context.graphics_queue)
                .expect("Unable to wait for queue to become idle!");

            self.vulkan_context
                .device
                .destroy_descriptor_set_layout(self.descriptor_set_layout, None);
            self.depth_image.destroy(&self.vulkan_context);
            self.vertex_buffer.destroy(&self.vulkan_context);
            self.uniform_buffer.destroy(&self.vulkan_context);
            self.index_buffer.destroy(&self.vulkan_context); // possible to get child resources to drop on their own??
            for frame in self.frames.drain(..) {
                frame.destroy(&self.vulkan_context);
            }
            self.vulkan_context
                .device
                .destroy_pipeline_layout(self.pipeline_layout, None);
            self.vulkan_context
                .device
                .destroy_render_pass(self.render_pass, None);
            self.vulkan_context
                .device
                .destroy_pipeline(self.pipeline, None);
        }
    }
}

impl Renderer {
    pub(crate) fn new(
        vulkan_context: VulkanContext,
        xr_swapchain: &xr::Swapchain<VulkanLegacy>,
        swapchain_resolution: vk::Extent2D,
        params: &ProgramInitialization,
    ) -> Result<Self> {
        println!("[HOTHAM_INIT] Creating renderer..");

        // Build swapchain
        let swapchain = Swapchain::new(xr_swapchain, swapchain_resolution)?;
        let render_area = vk::Rect2D {
            extent: swapchain.resolution,
            offset: vk::Offset2D::default(),
        };

        let descriptor_set_layout = create_descriptor_set_layout(&vulkan_context)?;

        // Pipeline, render pass
        let render_pass = create_render_pass(&vulkan_context)?;
        let pipeline_layout = create_pipeline_layout(&vulkan_context, &[descriptor_set_layout])?;
        let pipeline = create_pipeline(
            &vulkan_context,
            pipeline_layout,
            params,
            &render_area,
            render_pass,
        )?;

        // Depth image, shared between frames
        let depth_image = vulkan_context.create_image(DEPTH_FORMAT, &swapchain.resolution)?;

        // Create all the per-frame resources we need
        let frames = create_frames(&vulkan_context, &render_pass, &swapchain, &depth_image)?;

        // Create buffers
        let vertex_buffer = Buffer::new_from_vec(
            &vulkan_context,
            &params.vertices,
            vk::BufferUsageFlags::VERTEX_BUFFER,
        )?;
        let index_buffer = Buffer::new_from_vec(
            &vulkan_context,
            &params.indices,
            vk::BufferUsageFlags::INDEX_BUFFER,
        )?;

        let view_matrix = UniformBufferObject::default();
        let uniform_buffer = Buffer::new(
            &vulkan_context,
            &view_matrix,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        let uniform_buffer_descriptor_set =
            uniform_buffer.create_descriptor_set(&vulkan_context, &[descriptor_set_layout])?;

        println!("[HOTHAM_INIT] Done! Renderer initialised!");

        Ok(Self {
            _swapchain: swapchain,
            vulkan_context,
            frames,
            descriptor_set_layout,
            pipeline,
            pipeline_layout,
            render_pass,
            frame_index: 0,
            depth_image,
            render_area,
            vertex_buffer,
            index_buffer,
            uniform_buffer,
            uniform_buffer_descriptor_set,
            render_start_time: Instant::now(),
            cameras: vec![Default::default(); 2],
            term: Term::buffered_stdout(),
            views: Vec::new(),
        })
    }

    pub fn draw(&mut self, frame_index: usize) -> Result<()> {
        self.frame_index += 1;

        self.show_debug_info()?;

        let device = &self.vulkan_context.device;
        let frame = &self.frames[frame_index];

        self.prepare_frame(frame)?;

        let command_buffer = frame.command_buffer;
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
        let fence = frame.fence;
        let fences = [fence];

        unsafe {
            device.reset_fences(&fences)?;
            device.queue_submit(self.vulkan_context.graphics_queue, &[submit_info], fence)?;
            device.wait_for_fences(&fences, true, u64::MAX)?;
        };

        Ok(())
    }

    pub fn update_uniform_buffer(&mut self, views: &Vec<xr::View>, _rotation: f32) -> Result<()> {
        self.views = views.clone();

        let delta_time = Instant::now()
            .duration_since(self.render_start_time)
            .as_secs_f32();

        let scale = Matrix4::from_scale(0.2);
        let rotation_y = Matrix4::from_angle_y(Deg(10.0));
        let position = vec3(0.0, 0.0, 0.0);

        let translate = Matrix4::from_translation(position);
        let model = translate * rotation_y * scale;

        // View (camera)
        let view_matrices = &self
            .cameras
            .iter_mut()
            .enumerate()
            .map(|(n, c)| c.update_view_matrix(&views[n], delta_time))
            .collect::<Result<Vec<_>>>()?;

        // Projection
        let near = 0.05;
        let far = 100.0;
        // HACK - should check the imageArrayIndex
        let mvp = [
            get_projection(views[0].fov, near, far) * view_matrices[0] * model,
            get_projection(views[1].fov, near, far) * view_matrices[1] * model,
        ];

        let uniform_buffer = UniformBufferObject { mvp, delta_time };

        self.uniform_buffer
            .update(&self.vulkan_context, &uniform_buffer, 1)?;

        Ok(())
    }

    pub fn prepare_frame(&self, frame: &Frame) -> Result<()> {
        let device = &self.vulkan_context.device;
        let command_buffer = frame.command_buffer;
        let framebuffer = frame.framebuffer;
        let render_pass_begin_info = vk::RenderPassBeginInfo::builder()
            .render_pass(self.render_pass)
            .framebuffer(framebuffer)
            .render_area(self.render_area)
            .clear_values(&[
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
            ]);

        unsafe {
            device.begin_command_buffer(
                command_buffer,
                &vk::CommandBufferBeginInfo::builder()
                    .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
            )?;
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
                &[self.uniform_buffer_descriptor_set],
                &[],
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[self.vertex_buffer.handle], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                self.index_buffer.handle,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_draw_indexed(
                command_buffer,
                self.index_buffer.item_count as _,
                1,
                0,
                0,
                1,
            );
            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;
        };

        Ok(())
    }

    pub fn update(&self, _vertices: &Vec<Vertex>, _indices: &Vec<u32>) -> () {
        // println!("[HOTHAM_TEST] Vertices are now: {:?}", vertices);
        // println!("[HOTHAM_TEST] Indices are now: {:?}", indices);
    }

    fn show_debug_info(&self) -> Result<()> {
        if self.frame_index % 72 != 0 {
            return Ok(());
        };
        let e1 = Euler::from(self.cameras[0].orientation);
        let e2 = Euler::from(self.cameras[1].orientation);

        self.term.clear_screen()?;
        self.term.write_line("[RENDER_DEBUG]")?;
        self.term
            .write_line(&format!("[Frame]: {}", self.frame_index))?;
        self.term.write_line(&format!(
            "[Camera 0 Position]: {:?}",
            self.cameras[0].position
        ))?;
        self.term.write_line(&format!(
            "[Camera 0 Orientation]: Pitch {:?}, Yaw: {:?}, Roll: {:?}",
            Deg::from(e1.x),
            Deg::from(e1.y),
            Deg::from(e1.z)
        ))?;
        self.term.write_line(&format!(
            "[Camera 0 Matrix]: {:?}",
            self.cameras[0].view_matrix
        ))?;
        self.term.write_line(&format!(
            "[Camera 1 Position]: {:?}",
            self.cameras[1].position
        ))?;
        self.term.write_line(&format!(
            "[Camera 1 Orientation]: Pitch {:?}, Yaw: {:?}, Roll: {:?}",
            Deg::from(e2.x),
            Deg::from(e2.y),
            Deg::from(e2.z)
        ))?;
        self.term.write_line(&format!(
            "[Camera 1 Matrix]: {:?}",
            self.cameras[1].view_matrix
        ))?;
        let view = &self.views[0];
        let camera_position: mint::Vector3<f32> = view.pose.position.into();
        let angle_left = view.fov.angle_left;
        let angle_right = view.fov.angle_right;
        let angle_up = view.fov.angle_up;
        let angle_down = view.fov.angle_down;
        self.term.write_line(&format!(
            "[View 0]: Position: {:?}, angleLeft: {}, angleRight: {}, angleDown: {}, angleUp: {}",
            camera_position, angle_left, angle_right, angle_up, angle_down
        ))?;
        self.term.flush()?;

        Ok(())
    }
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

    return Matrix4::new(
        c0r0, c0r1, c0r2, c0r3, c1r0, c1r1, c1r2, c1r3, c2r0, c2r1, c2r2, c2r3, c3r0, c3r1, c3r2,
        c3r3,
    );
}

fn create_descriptor_set_layout(
    vulkan_context: &VulkanContext,
) -> VkResult<vk::DescriptorSetLayout> {
    let binding = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();
    let bindings = [binding];
    let create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&bindings);

    unsafe {
        vulkan_context
            .device
            .create_descriptor_set_layout(&create_info, None)
    }
}

fn create_frames(
    vulkan_context: &VulkanContext,
    render_pass: &vk::RenderPass,
    swapchain: &Swapchain,
    depth_image: &Image,
) -> Result<Vec<Frame>> {
    print!("[HOTHAM_INIT] Creating frames..");
    let frames = swapchain
        .images
        .iter()
        .flat_map(|i| vulkan_context.create_image_view(i, COLOR_FORMAT))
        .map(|i| {
            Frame::new(
                vulkan_context,
                *render_pass,
                swapchain.resolution,
                i,
                depth_image.view,
            )
        })
        .collect::<Result<Vec<Frame>>>()?;
    println!(" ..done!");
    Ok(frames)
}

fn create_render_pass(vulkan_context: &VulkanContext) -> Result<vk::RenderPass> {
    print!("[HOTHAM_INIT] Creating render pass..");
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

    let depth_stencil_reference = vk::AttachmentReference::builder()
        .attachment(1)
        .layout(vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL);

    let subpass = vk::SubpassDescription::builder()
        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
        .color_attachments(&color_attachments)
        .depth_stencil_attachment(&depth_stencil_reference)
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

    let view_masks = [!(!0 << VIEW_COUNT)];
    let mut multiview = vk::RenderPassMultiviewCreateInfo::builder()
        .view_masks(&view_masks)
        .correlation_masks(&view_masks);

    let create_info = vk::RenderPassCreateInfo::builder()
        .attachments(&attachments)
        .subpasses(&subpasses)
        .dependencies(&dependencies)
        .push_next(&mut multiview);

    let render_pass = unsafe { vulkan_context.device.create_render_pass(&create_info, None) }?;
    println!("..done!");

    Ok(render_pass)
}

fn create_pipeline(
    vulkan_context: &VulkanContext,
    pipeline_layout: vk::PipelineLayout,
    params: &ProgramInitialization,
    render_area: &vk::Rect2D,
    render_pass: vk::RenderPass,
) -> Result<vk::Pipeline> {
    print!("[HOTHAM_INIT] Creating pipeline..");
    // Build up the state of the pipeline

    // Vertex shader stage
    let vertex_code = &params.vertex_shader;
    let vertex_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);
    let vertex_shader = unsafe {
        vulkan_context
            .device
            .create_shader_module(&vertex_shader_create_info, None)
    }?;

    let main = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };

    let vertex_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::VERTEX)
        .name(main.clone())
        .module(vertex_shader)
        .build();

    // Fragment shader stage
    let fragment_code = &params.fragment_shader;
    let fragment_shader_create_info = vk::ShaderModuleCreateInfo::builder().code(&fragment_code);
    let fragment_shader = unsafe {
        vulkan_context
            .device
            .create_shader_module(&fragment_shader_create_info, None)
    }?;
    let fragment_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(vk::ShaderStageFlags::FRAGMENT)
        .name(main)
        .module(fragment_shader)
        .build();

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
        .front_face(vk::FrontFace::CLOCKWISE)
        .rasterizer_discard_enable(false)
        .depth_clamp_enable(false)
        .depth_bias_enable(false)
        .depth_bias_constant_factor(0.0)
        .depth_bias_clamp(0.0)
        .depth_bias_slope_factor(0.0)
        .line_width(1.0);

    // Multisample state
    let multisample_state = vk::PipelineMultisampleStateCreateInfo::builder()
        .rasterization_samples(vk::SampleCountFlags::TYPE_1);

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

    let mut pipelines = unsafe {
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

    println!(".. done!");

    pipelines.pop().ok_or(HothamError::EmptyListError.into())
}

fn create_pipeline_layout(
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
) -> Result<vk::PipelineLayout> {
    unsafe {
        vulkan_context.device.create_pipeline_layout(
            &vk::PipelineLayoutCreateInfo::builder().set_layouts(set_layouts),
            None,
        )
    }
    .map_err(|e| e.into())
}
