use anyhow::Result;
use hotham::{
    ash,
    contexts::{render_context::create_shader, VulkanContext},
    rendering::{buffer::Buffer, resources::QuadricData, vertex::Vertex},
    vk, Engine,
};
use std::{mem::size_of, slice};
use vk_shader_macros::include_glsl;

static QUADRIC_VERT: &[u32] =
    include_glsl!("../../hotham/src/shaders/quadric.vert", target: vulkan1_1);
static QUADRIC_FRAG: &[u32] =
    include_glsl!("../../hotham/src/shaders/quadric.frag", target: vulkan1_1);

// TODO: Ensure that this index doesn't collide with hotham engine internals.
pub const QUADRIC_DATA_BINDING: u32 = 0;
static QUADRIC_DATA_BUFFER_SIZE: usize = 100_000;

pub struct CustomRenderContext {
    /// Pipeline for drawing quadrics
    pub quadrics_pipeline: vk::Pipeline,
    pub quadrics_pipeline_layout: vk::PipelineLayout,
    /// Data for the holographic primitives that will be drawn this frame, indexed by gl_InstanceId
    pub quadrics_data_buffer: Buffer<QuadricData>,
    /// Descriptors for quadrics pipeline
    pub quadrics_descriptor_set_layout: vk::DescriptorSetLayout,
    pub quadrics_descriptor_set: vk::DescriptorSet,
}

impl CustomRenderContext {
    pub fn new(engine: &mut Engine) -> Self {
        let render_context = &mut engine.render_context;
        let vulkan_context = &engine.vulkan_context;
        let device = &vulkan_context.device;
        let quadrics_descriptor_set_layout = create_quadrics_descriptor_set_layout(device);
        let layouts = [
            render_context.descriptors.graphics_layout,
            quadrics_descriptor_set_layout,
        ];
        let create_info = &vk::PipelineLayoutCreateInfo::builder().set_layouts(&layouts);
        let quadrics_pipeline_layout =
            unsafe { device.create_pipeline_layout(create_info, None).unwrap() };
        let quadrics_pipeline = create_quadrics_pipeline(
            vulkan_context,
            quadrics_pipeline_layout,
            &render_context.render_area(),
            render_context.render_pass,
        )
        .unwrap();
        let quadrics_descriptor_set = unsafe {
            vulkan_context
                .device
                .allocate_descriptor_sets(
                    &vk::DescriptorSetAllocateInfo::builder()
                        .descriptor_pool(render_context.descriptors.pool)
                        .set_layouts(slice::from_ref(&quadrics_descriptor_set_layout)),
                )
                .unwrap()[0]
        };
        let quadrics_data_buffer = unsafe {
            Buffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                QUADRIC_DATA_BUFFER_SIZE,
            )
        };
        unsafe {
            quadrics_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                quadrics_descriptor_set,
                QUADRIC_DATA_BINDING,
            );
        }
        Self {
            quadrics_pipeline,
            quadrics_pipeline_layout,
            quadrics_data_buffer,
            quadrics_descriptor_set_layout,
            quadrics_descriptor_set,
        }
    }
}

fn create_quadrics_descriptor_set_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
    let quadrics_bindings = [
        // Quadric Data
        vk::DescriptorSetLayoutBinding {
            binding: QUADRIC_DATA_BINDING,
            descriptor_type: vk::DescriptorType::STORAGE_BUFFER,
            stage_flags: vk::ShaderStageFlags::VERTEX | vk::ShaderStageFlags::FRAGMENT,
            descriptor_count: 1,
            ..Default::default()
        },
    ];

    let descriptor_flags = [vk::DescriptorBindingFlags::empty()];
    let mut binding_flags = vk::DescriptorSetLayoutBindingFlagsCreateInfoEXT::builder()
        .binding_flags(&descriptor_flags);

    unsafe {
        device.create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&quadrics_bindings)
                .push_next(&mut binding_flags)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL),
            None,
        )
    }
    .unwrap()
}

fn create_quadrics_pipeline(
    vulkan_context: &VulkanContext,
    pipeline_layout: vk::PipelineLayout,
    render_area: &vk::Rect2D,
    render_pass: vk::RenderPass,
) -> Result<vk::Pipeline> {
    // Quadric vertex shader stage
    let (quadric_vertex_shader, quadric_vertex_stage) =
        create_shader(QUADRIC_VERT, vk::ShaderStageFlags::VERTEX, vulkan_context)?;

    // Quadric fragment shader stage
    let (quadric_fragment_shader, quadric_fragment_stage) =
        create_shader(QUADRIC_FRAG, vk::ShaderStageFlags::FRAGMENT, vulkan_context)?;

    let quadric_stages = [quadric_vertex_stage, quadric_fragment_stage];

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
        .depth_compare_op(vk::CompareOp::GREATER)
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

    let quadric_create_info = vk::GraphicsPipelineCreateInfo::builder()
        .stages(&quadric_stages)
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

    let create_infos = [quadric_create_info];

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
            .destroy_shader_module(quadric_vertex_shader, None);
        vulkan_context
            .device
            .destroy_shader_module(quadric_fragment_shader, None);
    }

    Ok(pipelines[0])
}
