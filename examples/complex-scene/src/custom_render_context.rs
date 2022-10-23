use anyhow::Result;
use hotham::{
    components::{skin::NO_SKIN, stage, GlobalTransform, Mesh, Skin, Visible},
    contexts::{
        render_context::{
            create_shader, Instance, InstancedPrimitive, InstancedQuadricPrimitive, QuadricInstance,
        },
        vulkan_context, RenderContext, VulkanContext,
    },
    glam::{Affine3A, Mat4},
    hecs::{With, World},
    rendering::{
        buffer::Buffer,
        resources::{DrawData, PrimitiveCullData, QuadricData, ShaderIndex},
        vertex::Vertex,
    },
    vk, xr, Engine,
};
use std::{collections::HashMap, ffi::CStr, mem::size_of, slice::from_ref as slice_from_ref};

static QUADRIC_VERT: &[u32] =
    include_glsl!("../../hotham/src/shaders/quadric.vert", target: vulkan1_1);
static QUADRIC_FRAG: &[u32] =
    include_glsl!("../../hotham/src/shaders/quadric.frag", target: vulkan1_1);

// TODO: Ensure that this index doesn't collide with hotham engine internals.
pub const QUADRIC_DATA_BINDING: u32 = 6;
static QUADRIC_DATA_BUFFER_SIZE: usize = 100_000;

pub struct CustomRenderContext {
    /// Pipeline for drawing quadrics
    pub quadrics_pipeline: vk::Pipeline,
    /// Data for the holographic primitives that will be drawn this frame, indexed by gl_InstanceId
    pub quadric_data_buffer: Buffer<QuadricData>,
    /// Descriptors for quadrics pipeline
    pub quadrics_descriptor_set_layout: vk::DescriptorSetLayout,
}

impl CustomRenderContext {
    pub fn new(&mut engine: Engine) -> Self {
        let vulkan_context = &mut engine.vulkan_context;
        let render_context = &mut engine.render_context;
        let device = &mut vulkan_context.device;
        let quadric_data_buffer = unsafe {
            Buffer::new(
                vulkan_context,
                vk::BufferUsageFlags::STORAGE_BUFFER,
                QUADRIC_DATA_BUFFER_SIZE,
            )
        };

        let quadrics_descriptor_layout = create_quadrics_descriptor_layout(device);
        let layouts = [
            render_context.descriptors.graphics_layout,
            quadrics_descriptor_layout,
        ];
        let create_info = &vk::PipelineLayoutCreateInfo::builder().set_layouts(&layouts);
        let quadrics_pipeline_layout = device.create_pipeline_layout(create_info, None).unwrap();

        let quadrics_pipeline = create_quadrics_pipeline(
            vulkan_context,
            quadrics_pipeline_layout,
            render_context.render_area(),
            render_context.render_pass,
        );
        unsafe {
            quadric_data_buffer.update_descriptor_set(
                &vulkan_context.device,
                descriptors.sets[0],
                QUADRIC_DATA_BINDING,
            );
        }
        Self {
            quadrics_pipeline,
            quadric_data_buffer,
            quadrics_descriptor_set_layout,
        }
    }
}

fn create_quadrics_descriptor_layout(device: &ash::Device) -> vk::DescriptorSetLayout {
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

    let quadrics_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder()
                .bindings(&quadrics_bindings)
                .push_next(&mut binding_flags)
                .flags(vk::DescriptorSetLayoutCreateFlags::UPDATE_AFTER_BIND_POOL),
            None,
        )
        .unwrap();

    let compute_layout = device
        .create_descriptor_set_layout(
            &vk::DescriptorSetLayoutCreateInfo::builder().bindings(&compute_bindings),
            None,
        )
        .unwrap();

    quadrics_layout
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
