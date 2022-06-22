use ash::vk::{self};
use egui::Pos2;
use itertools::izip;
use nalgebra::{vector, Vector2, Vector4};

use crate::components::mesh::MeshUBO;
use crate::components::{Material, Mesh, Primitive};
use crate::hotham_error::HothamError;
use crate::rendering::buffer::Buffer;
use crate::{
    rendering::texture::Texture,
    resources::{RenderContext, VulkanContext},
};
use crate::{rendering::vertex::Vertex, COLOR_FORMAT};

pub struct Panel {
    /// The resolution of the Panel
    pub resolution: vk::Extent2D,
    /// The world-size of the Panel
    pub world_size: Vector2<f32>,
    /// Texture backing the Panel
    pub texture: Texture,
    /// Input received this frame
    pub input: Option<PanelInput>,
}

impl Panel {
    pub fn create(
        vulkan_context: &VulkanContext,
        render_context: &mut RenderContext,
        resolution: vk::Extent2D,
        world_size: Vector2<f32>,
    ) -> Result<(Panel, Mesh), HothamError> {
        let output_image = vulkan_context
            .create_image(
                COLOR_FORMAT,
                &resolution,
                vk::ImageUsageFlags::COLOR_ATTACHMENT | vk::ImageUsageFlags::SAMPLED,
                1,
                1,
            )
            .unwrap();
        let sampler = vulkan_context
            .create_texture_sampler(vk::SamplerAddressMode::REPEAT, 1)
            .unwrap();
        let descriptor = vk::DescriptorImageInfo::builder()
            .image_layout(vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL)
            .image_view(output_image.view)
            .sampler(sampler)
            .build();
        let texture = Texture {
            image: output_image,
            sampler,
            descriptor,
        };
        todo!();
        let mesh = create_mesh(&texture, vulkan_context, render_context, world_size);

        Ok((
            Panel {
                resolution,
                world_size,
                texture,
                input: Default::default(),
            },
            mesh,
        ))
    }
}

fn create_mesh(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
    world_size: Vector2<f32>,
) -> Mesh {
    let material_id = add_material(output_texture, vulkan_context, render_context);
    let (half_width, half_height) = (world_size.x / 2., world_size.y / 2.);

    let positions = [
        vector![-half_width, half_height, 0.],  // v0
        vector![half_width, -half_height, 0.],  // v1
        vector![half_width, half_height, 0.],   // v2
        vector![-half_width, -half_height, 0.], // v3
    ];
    let tex_coords_0 = [
        vector![0., 0.], // v0
        vector![1., 1.], // v1
        vector![1., 0.], // v2
        vector![0., 1.], // v3
    ];
    let vertices: Vec<Vertex> = izip!(positions, tex_coords_0)
        .into_iter()
        .map(|(p, t)| Vertex {
            position: p,
            texture_coords_0: t,
            ..Default::default()
        })
        .collect();

    let indices = [0, 1, 2, 0, 3, 1];

    let primitive = Primitive {
        indices_count: 6,
        material_id,
        index_buffer_offset: render_context.resources.index_buffer.len,
        vertex_buffer_offset: render_context.resources.vertex_buffer.len,
    };

    unsafe {
        render_context.resources.index_buffer.append(&indices);
        render_context.resources.vertex_buffer.append(&vertices);
    }

    Mesh {
        primitives: vec![primitive],
    }
}

fn add_material(
    output_texture: &Texture,
    vulkan_context: &VulkanContext,
    render_context: &mut RenderContext,
) -> u32 {
    let material = Material::unlit_white();
    unsafe {
        render_context.resources.materials_buffer.push(&material);
    }
    render_context.resources.materials_buffer.len as _
}

/// Input to a panel
#[derive(Debug, Clone)]
pub struct PanelInput {
    /// Location of the cursor, in panel space
    pub cursor_location: Pos2,
    /// Value of the controller trigger
    pub trigger_value: f32,
}
