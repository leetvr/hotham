use std::{
    cell::RefCell, collections::HashMap, ffi::CStr, io::Cursor, mem::size_of, rc::Rc,
    time::Instant, u64,
};

use crate::{
    buffer::Buffer, camera::Camera, frame::Frame, gltf_loader::load_gltf_nodes, image::Image,
    node::Node, swapchain::Swapchain, vulkan_context::VulkanContext, UniformBufferObject, Vertex,
    COLOR_FORMAT, DEPTH_FORMAT, VIEW_COUNT,
};
use anyhow::Result;
use ash::{prelude::VkResult, version::DeviceV1_0, vk};
use cgmath::{vec4, Deg, Euler, Matrix4};
use openxr as xr;

use xr::VulkanLegacy;

pub(crate) struct Renderer {
    _swapchain: Swapchain,
    pub vulkan_context: VulkanContext,
    frames: Vec<Frame>,
    descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
    pipeline_layout: vk::PipelineLayout,
    pipeline: vk::Pipeline,
    render_pass: vk::RenderPass,
    depth_image: Image,
    render_area: vk::Rect2D,
    uniform_buffer: Buffer<UniformBufferObject>,
    render_start_time: Instant,
    cameras: Vec<Camera>,
    views: Vec<xr::View>,
    last_frame_time: Instant,
    pub frame_index: usize,
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            self.vulkan_context
                .device
                .queue_wait_idle(self.vulkan_context.graphics_queue)
                .expect("Unable to wait for queue to become idle!");

            for layout in &self.descriptor_set_layouts {
                self.vulkan_context
                    .device
                    .destroy_descriptor_set_layout(*layout, None);
            }
            self.depth_image.destroy(&self.vulkan_context);
            self.uniform_buffer.destroy(&self.vulkan_context);
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
    ) -> Result<Self> {
        println!("[HOTHAM_RENDERER] Creating renderer..");

        // Build swapchain
        let swapchain = Swapchain::new(xr_swapchain, swapchain_resolution)?;
        let render_area = vk::Rect2D {
            extent: swapchain.resolution,
            offset: vk::Offset2D::default(),
        };

        let descriptor_set_layouts = create_descriptor_set_layouts(&vulkan_context)?;

        // Pipeline, render pass
        let render_pass = create_render_pass(&vulkan_context)?;
        let pipeline_layout = create_pipeline_layout(&vulkan_context, &descriptor_set_layouts)?;
        let pipeline =
            create_pipeline(&vulkan_context, pipeline_layout, &render_area, render_pass)?;

        // Depth image, shared between frames
        let depth_image = vulkan_context.create_image(DEPTH_FORMAT, &swapchain.resolution)?;

        // Create all the per-frame resources we need
        let frames = create_frames(&vulkan_context, &render_pass, &swapchain, &depth_image)?;

        println!("[HOTHAM_RENDERER] Creating UBO..");
        let view_matrix = UniformBufferObject::default();
        let uniform_buffer = Buffer::new(
            &vulkan_context,
            &view_matrix,
            vk::BufferUsageFlags::UNIFORM_BUFFER,
        )?;
        println!("[HOTHAM_RENDERER] ..done! {:?}", uniform_buffer);

        println!("[HOTHAM_RENDERER] Done! Renderer initialised!");

        Ok(Self {
            _swapchain: swapchain,
            vulkan_context,
            frames,
            descriptor_set_layouts,
            pipeline,
            pipeline_layout,
            render_pass,
            frame_index: 0,
            depth_image,
            render_area,
            uniform_buffer,
            render_start_time: Instant::now(),
            cameras: vec![Default::default(); 2],
            views: Vec::new(),
            last_frame_time: Instant::now(),
        })
    }

    pub fn draw(&mut self, frame_index: usize, nodes: &Vec<Rc<RefCell<Node>>>) -> Result<()> {
        self.frame_index += 1;

        // self.show_debug_info(nodes)?;

        let device = &self.vulkan_context.device;
        let frame = &self.frames[frame_index];

        let command_buffer = frame.command_buffer;
        let submit_info = vk::SubmitInfo::builder()
            .command_buffers(&[command_buffer])
            .build();
        let fence = frame.fence;
        let fences = [fence];
        self.prepare_frame(frame, nodes)?;

        unsafe {
            device.reset_fences(&fences)?;
            device.queue_submit(self.vulkan_context.graphics_queue, &[submit_info], fence)?;
            device.wait_for_fences(&fences, true, u64::MAX)?;
        };

        // Update our frame timer
        self.last_frame_time = Instant::now();

        Ok(())
    }

    pub fn update_uniform_buffer(&mut self, views: &Vec<xr::View>) -> Result<()> {
        self.views = views.clone();
        let delta_time = Instant::now()
            .duration_since(self.render_start_time)
            .as_secs_f32();

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

        let view = [view_matrices[0], view_matrices[1]];

        let projection = [
            get_projection(views[0].fov, near, far),
            get_projection(views[1].fov, near, far),
        ];

        // let light_pos = vec4(2.0, 20.0 + delta_time, 10.0, 0.0);
        let light_pos = vec4(0.0, 2.0, 2.0, 1.0);

        let uniform_buffer = UniformBufferObject {
            view,
            projection,
            delta_time,
            light_pos,
        };

        self.uniform_buffer
            .update(&self.vulkan_context, &uniform_buffer, 1)?;

        Ok(())
    }

    pub fn prepare_frame(&self, frame: &Frame, nodes: &Vec<Rc<RefCell<Node>>>) -> Result<()> {
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

            for node in nodes {
                self.draw_node(&(**node).borrow(), device, command_buffer)?;
            }

            device.cmd_end_render_pass(command_buffer);
            device.end_command_buffer(command_buffer)?;
        };

        Ok(())
    }

    unsafe fn draw_node(
        &self,
        node: &Node,
        device: &ash::Device,
        command_buffer: vk::CommandBuffer,
    ) -> Result<()> {
        // Update any animations
        // node.update_animation(
        //     self.last_frame_time.elapsed().as_secs_f32(),
        //     &self.vulkan_context,
        // )?;

        if let Some(mesh) = node.mesh.as_ref() {
            // Bind mesh descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &mesh.descriptor_sets,
                &[],
            );

            // Bind vertex and index buffers
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[mesh.vertex_buffer.handle], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                mesh.index_buffer.handle,
                0,
                vk::IndexType::UINT32,
            );

            // Bind skin descriptor sets, if present.
            if let Some(skin) = node.skin.as_ref() {
                device.cmd_bind_descriptor_sets(
                    command_buffer,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.pipeline_layout,
                    1,
                    &skin.descriptor_sets,
                    &[],
                )
            }

            // Push constants
            let node_matrix = node.get_node_matrix();
            let node_matrix = create_push_constant(&node_matrix);

            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                node_matrix,
            );
            device.cmd_draw_indexed(command_buffer, mesh.num_indices, 1, 0, 0, 1);
        }

        for child in &node.children {
            let child = (*child).borrow();
            self.draw_node(&child, device, command_buffer)?;
        }

        Ok(())
    }

    pub(crate) fn load_gltf_nodes(
        &self,
        gltf_data: (&[u8], &[u8]),
    ) -> Result<HashMap<String, Rc<RefCell<Node>>>> {
        let buffers = vec![gltf_data.1];
        load_gltf_nodes(
            gltf_data.0,
            &buffers,
            &self.vulkan_context,
            &self.descriptor_set_layouts,
            self.uniform_buffer.handle,
        )
    }
}

fn create_push_constant<T: Sized>(p: &T) -> &[u8] {
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

    return Matrix4::new(
        c0r0, c0r1, c0r2, c0r3, c1r0, c1r1, c1r2, c1r3, c2r0, c2r1, c2r2, c2r3, c3r0, c3r1, c3r2,
        c3r3,
    );
}

pub(crate) fn create_descriptor_set_layouts(
    vulkan_context: &VulkanContext,
) -> VkResult<Vec<vk::DescriptorSetLayout>> {
    let uniform_buffer = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::UNIFORM_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let base_color_image_sampler = vk::DescriptorSetLayoutBinding::builder()
        .binding(1)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .build();

    let normal_image_sampler = vk::DescriptorSetLayoutBinding::builder()
        .binding(2)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
        .stage_flags(vk::ShaderStageFlags::FRAGMENT)
        .build();

    let mesh_bindings = [
        uniform_buffer,
        base_color_image_sampler,
        normal_image_sampler,
    ];
    let mesh_create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&mesh_bindings);

    let mesh_layout = unsafe {
        vulkan_context
            .device
            .create_descriptor_set_layout(&mesh_create_info, None)
    }?;

    let skin_joint_buffer = vk::DescriptorSetLayoutBinding::builder()
        .binding(0)
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
        .stage_flags(vk::ShaderStageFlags::VERTEX)
        .build();

    let skin_bindings = [skin_joint_buffer];
    let skin_create_info = vk::DescriptorSetLayoutCreateInfo::builder().bindings(&skin_bindings);

    let skin_layout = unsafe {
        vulkan_context
            .device
            .create_descriptor_set_layout(&skin_create_info, None)
    }?;

    Ok(vec![mesh_layout, skin_layout])
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
    render_area: &vk::Rect2D,
    render_pass: vk::RenderPass,
) -> Result<vk::Pipeline> {
    print!("[HOTHAM_INIT] Creating pipeline..");
    // Build up the state of the pipeline

    // Vertex shader stage
    let (vertex_shader, vertex_stage) = create_shader(
        include_bytes!("../shaders/model.vert.spv"),
        vk::ShaderStageFlags::VERTEX,
        vulkan_context,
    )?;

    // Fragment shader stage
    let (fragment_shader, fragment_stage) = create_shader(
        include_bytes!("../shaders/model.frag.spv"),
        vk::ShaderStageFlags::FRAGMENT,
        vulkan_context,
    )?;

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

fn create_shader(
    shader_code: &[u8],
    stage: vk::ShaderStageFlags,
    vulkan_context: &VulkanContext,
) -> Result<(vk::ShaderModule, vk::PipelineShaderStageCreateInfo)> {
    let mut cursor = Cursor::new(shader_code);
    let shader_code = ash::util::read_spv(&mut cursor)?;
    let main = unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") };
    let create_info = vk::ShaderModuleCreateInfo::builder().code(&shader_code);
    let shader_module = unsafe {
        vulkan_context
            .device
            .create_shader_module(&create_info, None)
    }?;
    let shader_stage = vk::PipelineShaderStageCreateInfo::builder()
        .stage(stage)
        .name(main.clone())
        .module(shader_module)
        .build();

    Ok((shader_module, shader_stage))
}

fn create_pipeline_layout(
    vulkan_context: &VulkanContext,
    set_layouts: &[vk::DescriptorSetLayout],
) -> Result<vk::PipelineLayout> {
    let push_constant_ranges = [vk::PushConstantRange {
        stage_flags: vk::ShaderStageFlags::VERTEX,
        offset: 0,
        size: size_of::<Matrix4<f32>>() as _,
    }];
    let create_info = &vk::PipelineLayoutCreateInfo::builder()
        .set_layouts(set_layouts)
        .push_constant_ranges(&push_constant_ranges);
    unsafe {
        vulkan_context
            .device
            .create_pipeline_layout(&create_info, None)
    }
    .map_err(|e| e.into())
}
