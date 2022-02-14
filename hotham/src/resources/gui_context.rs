use ash::vk::{self, Handle};

/// How much the GUI should be scaled by
// TODO - this this necessary?
pub const SCALE_FACTOR: f32 = 3.;

use crate::{
    components::{panel::PanelInput, Panel},
    resources::render_context::{create_push_constant, CLEAR_VALUES},
    texture::Texture,
    COLOR_FORMAT,
};

use super::{render_context::create_shader, RenderContext, VulkanContext};

/// Encapsulates egui state
/// Used by `update_gui_system`
#[derive(Debug, Clone)]
pub struct GuiContext {
    pub(crate) render_pass: vk::RenderPass,
    pub(crate) pipeline: vk::Pipeline,
    pub(crate) pipeline_layout: vk::PipelineLayout,
    pub(crate) font_texture_descriptor_sets: Vec<vk::DescriptorSet>,
    pub(crate) font_texture_version: u64,
    pub(crate) hovered_this_frame: bool,
    pub(crate) hovered_last_frame: bool,
}

impl GuiContext {
    /// Create a new GuiContext
    pub fn new(vulkan_context: &VulkanContext) -> Self {
        let device = &vulkan_context.device;

        // Descriptor sets, etc
        let descriptor_set_layout = unsafe {
            device
                .create_descriptor_set_layout(
                    &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&[
                        vk::DescriptorSetLayoutBinding::builder()
                            .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                            .descriptor_count(1)
                            .binding(0)
                            .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                            .build(),
                    ]),
                    None,
                )
                .expect("Failed to create descriptor set layout.")
        };
        let font_texture_descriptor_sets = unsafe {
            device.allocate_descriptor_sets(
                &vk::DescriptorSetAllocateInfo::builder()
                    .descriptor_pool(vulkan_context.descriptor_pool)
                    .set_layouts(&[descriptor_set_layout]),
            )
        }
        .expect("Failed to create descriptor sets.");

        // Create PipelineLayout
        let pipeline_layout = unsafe {
            device.create_pipeline_layout(
                &vk::PipelineLayoutCreateInfo::builder()
                    .set_layouts(&[descriptor_set_layout])
                    .push_constant_ranges(&[vk::PushConstantRange::builder()
                        .stage_flags(vk::ShaderStageFlags::VERTEX)
                        .offset(0)
                        .size(std::mem::size_of::<f32>() as u32 * 2) // screen size
                        .build()]),
                None,
            )
        }
        .expect("Failed to create pipeline layout.");

        vulkan_context
            .set_debug_name(
                vk::ObjectType::PIPELINE_LAYOUT,
                pipeline_layout.as_raw(),
                "GUI Pipeline Layout",
            )
            .unwrap();

        // Create render pass
        let render_pass = unsafe {
            device.create_render_pass(
                &vk::RenderPassCreateInfo::builder()
                    .attachments(&[vk::AttachmentDescription::builder()
                        .format(COLOR_FORMAT)
                        .samples(vk::SampleCountFlags::TYPE_1)
                        .load_op(vk::AttachmentLoadOp::CLEAR)
                        .store_op(vk::AttachmentStoreOp::STORE)
                        .stencil_load_op(vk::AttachmentLoadOp::DONT_CARE)
                        .stencil_store_op(vk::AttachmentStoreOp::DONT_CARE)
                        .initial_layout(vk::ImageLayout::UNDEFINED)
                        .final_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
                        .build()])
                    .subpasses(&[vk::SubpassDescription::builder()
                        .pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS)
                        .color_attachments(&[vk::AttachmentReference::builder()
                            .attachment(0)
                            .layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                            .build()])
                        .build()])
                    .dependencies(&[vk::SubpassDependency::builder()
                        .src_subpass(0)
                        .dst_subpass(vk::SUBPASS_EXTERNAL)
                        .src_access_mask(
                            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                        )
                        .dst_access_mask(
                            vk::AccessFlags::MEMORY_READ | vk::AccessFlags::MEMORY_WRITE,
                        )
                        .src_stage_mask(vk::PipelineStageFlags::ALL_GRAPHICS)
                        .dst_stage_mask(vk::PipelineStageFlags::ALL_GRAPHICS)
                        .build()]),
                None,
            )
        }
        .expect("Failed to create render pass.");

        // Create Pipeline
        let pipeline = {
            let bindings = [vk::VertexInputBindingDescription::builder()
                .binding(0)
                .input_rate(vk::VertexInputRate::VERTEX)
                .stride(
                    4 * std::mem::size_of::<f32>() as u32 + 4 * std::mem::size_of::<u8>() as u32,
                )
                .build()];

            let attributes = [
                // position
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(0)
                    .location(0)
                    .format(vk::Format::R32G32_SFLOAT)
                    .build(),
                // uv
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(8)
                    .location(1)
                    .format(vk::Format::R32G32_SFLOAT)
                    .build(),
                // color
                vk::VertexInputAttributeDescription::builder()
                    .binding(0)
                    .offset(16)
                    .location(2)
                    .format(vk::Format::R8G8B8A8_UNORM)
                    .build(),
            ];

            // Vertex shader stage
            let (vertex_shader, vertex_stage) = create_shader(
                include_bytes!("../../shaders/gui.vert.spv"),
                vk::ShaderStageFlags::VERTEX,
                vulkan_context,
            )
            .expect("Unable to create vertex shader");

            // Fragment shader stage
            let (fragment_shader, fragment_stage) = create_shader(
                include_bytes!("../../shaders/gui.frag.spv"),
                vk::ShaderStageFlags::FRAGMENT,
                vulkan_context,
            )
            .expect("Unable to create fragment shader");

            let pipeline_shader_stages = [vertex_stage, fragment_stage];

            let input_assembly_info = vk::PipelineInputAssemblyStateCreateInfo::builder()
                .topology(vk::PrimitiveTopology::TRIANGLE_LIST);
            let viewport_info = vk::PipelineViewportStateCreateInfo::builder()
                .viewport_count(1)
                .scissor_count(1);
            let rasterization_info = vk::PipelineRasterizationStateCreateInfo::builder()
                .depth_clamp_enable(false)
                .rasterizer_discard_enable(false)
                .polygon_mode(vk::PolygonMode::FILL)
                .cull_mode(vk::CullModeFlags::NONE)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                .depth_bias_enable(false)
                .line_width(1.0);
            let stencil_op = vk::StencilOpState::builder()
                .fail_op(vk::StencilOp::KEEP)
                .pass_op(vk::StencilOp::KEEP)
                .compare_op(vk::CompareOp::ALWAYS)
                .build();
            let depth_stencil_info = vk::PipelineDepthStencilStateCreateInfo::builder()
                .depth_test_enable(false)
                .depth_write_enable(false)
                .depth_compare_op(vk::CompareOp::ALWAYS)
                .depth_bounds_test_enable(false)
                .stencil_test_enable(false)
                .front(stencil_op)
                .back(stencil_op);
            let color_blend_attachments = [vk::PipelineColorBlendAttachmentState::builder()
                .color_write_mask(
                    vk::ColorComponentFlags::R
                        | vk::ColorComponentFlags::G
                        | vk::ColorComponentFlags::B
                        | vk::ColorComponentFlags::A,
                )
                .blend_enable(true)
                .src_color_blend_factor(vk::BlendFactor::ONE)
                .dst_color_blend_factor(vk::BlendFactor::ONE_MINUS_SRC_ALPHA)
                .build()];
            let color_blend_info = vk::PipelineColorBlendStateCreateInfo::builder()
                .attachments(&color_blend_attachments);
            let dynamic_states = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
            let dynamic_state_info =
                vk::PipelineDynamicStateCreateInfo::builder().dynamic_states(&dynamic_states);
            let vertex_input_state = vk::PipelineVertexInputStateCreateInfo::builder()
                .vertex_attribute_descriptions(&attributes)
                .vertex_binding_descriptions(&bindings);
            let multisample_info = vk::PipelineMultisampleStateCreateInfo::builder()
                .rasterization_samples(vk::SampleCountFlags::TYPE_1);

            let pipeline_create_info = [vk::GraphicsPipelineCreateInfo::builder()
                .stages(&pipeline_shader_stages)
                .vertex_input_state(&vertex_input_state)
                .input_assembly_state(&input_assembly_info)
                .viewport_state(&viewport_info)
                .rasterization_state(&rasterization_info)
                .multisample_state(&multisample_info)
                .depth_stencil_state(&depth_stencil_info)
                .color_blend_state(&color_blend_info)
                .dynamic_state(&dynamic_state_info)
                .layout(pipeline_layout)
                .render_pass(render_pass)
                .subpass(0)
                .build()];

            let pipeline = unsafe {
                device.create_graphics_pipelines(
                    vk::PipelineCache::null(),
                    &pipeline_create_info,
                    None,
                )
            }
            .expect("Failed to create graphics pipeline.")[0];
            unsafe {
                device.destroy_shader_module(vertex_shader, None);
                device.destroy_shader_module(fragment_shader, None);
            }
            vulkan_context
                .set_debug_name(vk::ObjectType::PIPELINE, pipeline.as_raw(), "GUI Pipeline")
                .unwrap();
            pipeline
        };

        Self {
            render_pass,
            pipeline,
            pipeline_layout,
            font_texture_descriptor_sets,
            font_texture_version: 0,
            hovered_last_frame: false,
            hovered_this_frame: false,
        }
    }

    pub(crate) fn paint_gui(
        &mut self,
        vulkan_context: &VulkanContext,
        render_context: &RenderContext,
        current_swapchain_image_index: usize,
        panel: &mut Panel,
    ) {
        let device = &vulkan_context.device;
        let frame = &render_context.frames[current_swapchain_image_index];
        let command_buffer = frame.command_buffer;
        let framebuffer = panel.framebuffer;
        let extent = panel.extent;
        let (raw_input, panel_input) = handle_panel_input(panel);

        let text = panel.text.clone();
        let mut updated_buttons = panel.buttons.clone();
        let egui_context = &mut panel.egui_context;

        egui_context.begin_frame(raw_input);
        let inner_layout = egui::Layout::from_main_dir_and_cross_align(
            egui::Direction::TopDown,
            egui::Align::Center,
        );

        // GUI Layout
        egui::CentralPanel::default().show(egui_context, |ui| {
            ui.with_layout(inner_layout, |ui| {
                ui.heading(&text);

                for button in &mut updated_buttons {
                    let response = ui.button(&button.text);

                    if response.hovered() {
                        self.hovered_this_frame = true;
                    }

                    if response.clicked() {
                        button.clicked_this_frame = true;
                    }
                }

                if let Some(panel_input) = panel_input {
                    let position = ui
                        .painter()
                        .round_pos_to_pixels(panel_input.cursor_location);
                    let cursor_colour = if panel_input.trigger_value > 0.9 {
                        egui::Color32::LIGHT_BLUE
                    } else {
                        egui::Color32::WHITE
                    };
                    ui.painter().circle_filled(position, 4.00, cursor_colour);
                }
                ui.allocate_space(ui.available_size())
            })
        });

        let (_output, shapes) = egui_context.end_frame();

        let texture = &egui_context.fonts().texture();
        if texture.version != self.font_texture_version {
            let _font_texture = update_font_texture(
                vulkan_context,
                texture,
                self.font_texture_descriptor_sets[0],
            );
            self.font_texture_version = texture.version;
        }

        let clipped_meshes = egui_context.tessellate(shapes);
        panel.buttons = updated_buttons;
        let vertex_buffer = &panel.vertex_buffer;
        let index_buffer = &panel.index_buffer;

        // begin render pass
        unsafe {
            device.cmd_begin_render_pass(
                command_buffer,
                &vk::RenderPassBeginInfo::builder()
                    .render_pass(self.render_pass)
                    .framebuffer(framebuffer)
                    .clear_values(&[CLEAR_VALUES[0]])
                    .render_area(vk::Rect2D::builder().extent(extent).build()),
                vk::SubpassContents::INLINE,
            );
            device.cmd_bind_pipeline(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline,
            );
            device.cmd_bind_vertex_buffers(command_buffer, 0, &[vertex_buffer.handle], &[0]);
            device.cmd_bind_index_buffer(
                command_buffer,
                index_buffer.handle,
                0,
                vk::IndexType::UINT32,
            );
            device.cmd_set_viewport(
                command_buffer,
                0,
                &[vk::Viewport::builder()
                    .x(0.0)
                    .y(0.0)
                    .width(extent.width as f32)
                    .height(extent.height as f32)
                    .min_depth(0.0)
                    .max_depth(1.0)
                    .build()],
            );

            // Set push contants
            let width_points = extent.width as f32 / SCALE_FACTOR;
            let height_points = extent.height as f32 / SCALE_FACTOR;
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                0,
                create_push_constant(&width_points),
            );
            device.cmd_push_constants(
                command_buffer,
                self.pipeline_layout,
                vk::ShaderStageFlags::VERTEX,
                std::mem::size_of_val(&width_points) as u32,
                create_push_constant(&height_points),
            );

            // Bind descriptor sets
            device.cmd_bind_descriptor_sets(
                command_buffer,
                vk::PipelineBindPoint::GRAPHICS,
                self.pipeline_layout,
                0,
                &self.font_texture_descriptor_sets,
                &[],
            );
        }

        for egui::ClippedMesh(rect, mesh) in &clipped_meshes {
            // Update vertex buffer
            vertex_buffer
                .update(vulkan_context, &mesh.vertices)
                .unwrap();

            // Update index buffer
            index_buffer.update(vulkan_context, &mesh.indices).unwrap();

            // record draw commands
            unsafe {
                let width = extent.width as f32;
                let height = extent.height as f32;

                let min = rect.min;
                let min = egui::Pos2 {
                    x: min.x * SCALE_FACTOR,
                    y: min.y * SCALE_FACTOR,
                };
                let min = egui::Pos2 {
                    x: f32::clamp(min.x, 0.0, width),
                    y: f32::clamp(min.y, 0.0, height),
                };
                let max = rect.max;
                let max = egui::Pos2 {
                    x: max.x * SCALE_FACTOR,
                    y: max.y * SCALE_FACTOR,
                };
                let max = egui::Pos2 {
                    x: f32::clamp(max.x, min.x, width),
                    y: f32::clamp(max.y, min.y, height),
                };
                device.cmd_set_scissor(
                    command_buffer,
                    0,
                    &[vk::Rect2D::builder()
                        .offset(
                            vk::Offset2D::builder()
                                .x(min.x.round() as i32)
                                .y(min.y.round() as i32)
                                .build(),
                        )
                        .extent(
                            vk::Extent2D::builder()
                                .width((max.x.round() - min.x) as u32)
                                .height((max.y.round() - min.y) as u32)
                                .build(),
                        )
                        .build()],
                );

                device.cmd_draw_indexed(command_buffer, mesh.indices.len() as u32, 1, 0, 0, 0);
            }
        }

        unsafe {
            device.cmd_end_render_pass(command_buffer);
        }
    }
}

fn handle_panel_input(panel: &mut Panel) -> (egui::RawInput, Option<PanelInput>) {
    let mut raw_input = panel.raw_input.clone();
    let panel_input = panel.input.take();
    if let Some(input) = &panel_input {
        let pos = input.cursor_location;
        raw_input.events.push(egui::Event::PointerMoved(pos));
        if input.trigger_value >= 0. {
            raw_input.events.push(egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: input.trigger_value > 0.9,
                modifiers: Default::default(),
            });
        }
    } else {
        raw_input.events.push(egui::Event::PointerGone);
    }

    (raw_input, panel_input)
}

fn update_font_texture(
    vulkan_context: &VulkanContext,
    texture: &egui::Texture,
    descriptor_set: vk::DescriptorSet,
) -> Texture {
    unsafe {
        vulkan_context
            .device
            .device_wait_idle()
            .expect("Failed to wait device idle");
    }

    let image_buf = texture
        .pixels
        .iter()
        .flat_map(|&r| vec![r, r, r, r])
        .collect::<Vec<_>>();

    let texture = Texture::new(
        "Font texture",
        vulkan_context,
        &image_buf,
        texture.width as u32,
        texture.height as u32,
        COLOR_FORMAT,
    )
    .expect("Unable to create font texture");

    unsafe {
        vulkan_context.device.update_descriptor_sets(
            &[vk::WriteDescriptorSet::builder()
                .descriptor_type(vk::DescriptorType::COMBINED_IMAGE_SAMPLER)
                .dst_set(descriptor_set)
                .image_info(&[texture.descriptor])
                .dst_binding(0)
                .build()],
            &[],
        );
    }

    texture
}
